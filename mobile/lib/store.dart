import 'dart:async';
import 'dart:convert';

import 'package:device_info_plus/device_info_plus.dart';
import 'package:flutter/widgets.dart';
import 'package:path_provider/path_provider.dart';

import 'models.dart';
import 'src/rust/api/spof.dart' as rust;

/// Estado do app: lê do banco local (via bridge) e notifica as telas.
class Peer {
  final String name, host;
  final int port;
  Peer(this.name, this.host, this.port);
}

class Store extends ChangeNotifier with WidgetsBindingObserver {
  SystemStatus? status;
  List<Course> courses = [];
  List<Deadline> deadlines = [];
  List<OutboxItem> outbox = [];
  SessionInfo session = SessionInfo.fromJson({'is_authenticated': false, 'base_url': null});
  String version = '';
  bool syncing = false;
  String? lastError;
  String? lastSyncMessage;
  Peer? peer;
  bool pcSyncing = false;
  String? pcSyncError;
  DateTime? pcLastSync;
  String deviceName = 'Celular';
  Timer? _timer;

  Future<void> init() async {
    final dir = await getApplicationDocumentsDirectory();
    await rust.openDatabase(path: '${dir.path}/0spof.db');
    version = await rust.appVersion();
    try {
      final a = await DeviceInfoPlugin().androidInfo;
      deviceName = '${a.manufacturer} ${a.model}'.trim();
    } catch (_) {}
    await refresh();
    WidgetsBinding.instance.addObserver(this);
    _timer = Timer.periodic(const Duration(minutes: 1), (_) => syncPc(silent: true));
    unawaited(syncPc(silent: true));
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) unawaited(syncPc(silent: true));
  }

  @override
  void dispose() {
    _timer?.cancel();
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  Future<void> _loadPeer() async {
    final j = jsonDecode(await rust.peerInfo());
    peer = j == null ? null : Peer(j['name'] ?? '', j['host'], j['port']);
  }

  /// Pareia com o PC (conteúdo do QR code) e já sincroniza. Lança em caso de erro.
  Future<void> pairWithPc(String code) async {
    await rust.pair(code: code, device: deviceName);
    pcLastSync = DateTime.now();
    pcSyncError = null;
    await refresh();
  }

  Future<void> unpairPc() async {
    await rust.unpair();
    pcSyncError = null;
    await refresh();
  }

  /// Sincroniza com o PC. Em modo `silent` (automático), falhar fora de casa não incomoda.
  Future<void> syncPc({bool silent = false}) async {
    if (pcSyncing) return;
    await _loadPeer();
    if (peer == null) {
      notifyListeners();
      return;
    }
    pcSyncing = true;
    notifyListeners();
    try {
      await rust.syncPeer(device: deviceName);
      pcLastSync = DateTime.now();
      pcSyncError = null;
    } catch (e) {
      if (!silent) pcSyncError = '$e';
    }
    pcSyncing = false;
    await refresh();
  }

  Future<void> refresh() async {
    try {
      status = SystemStatus.parse(await rust.systemStatus());
      courses = Course.parseList(await rust.courses());
      deadlines = Deadline.parseList(await rust.upcomingDeadlines());
      outbox = OutboxItem.parseList(await rust.outboxItems());
      session = SessionInfo.parse(await rust.sessionInfo());
      await _loadPeer();
      lastError = null;
    } catch (e) {
      lastError = '$e';
    }
    notifyListeners();
  }

  Future<List<Section>> courseDetails(int id) async =>
      Section.parseList(await rust.courseDetails(courseId: id));

  Future<void> setMuted(int courseId, bool muted) async {
    await rust.setCourseMuted(courseId: courseId, isMuted: muted);
    await refresh();
  }

  Future<void> syncMoodle() async {
    if (syncing) return;
    syncing = true;
    lastError = null;
    notifyListeners();
    try {
      final s = SyncSummary.parse(await rust.syncMoodle());
      lastSyncMessage =
          '${s.courses} disciplinas, ${s.activities} atividades, ${s.resources} materiais';
    } catch (e) {
      lastError = '$e';
    }
    syncing = false;
    await refresh();
  }
}
