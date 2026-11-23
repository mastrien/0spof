import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:device_info_plus/device_info_plus.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:ota_update/ota_update.dart';
import 'package:path_provider/path_provider.dart';

import 'models.dart';
import 'native_cookies.dart';
import 'updater.dart';
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
  AppUpdate? update;
  int? updateProgress;
  String? updateError;
  final _secure = const FlutterSecureStorage();

  static const defaultMoodleUrl = 'https://moodle2.jcr.ifsp.edu.br';
  static const _kBaseUrl = 'moodle_base_url';
  static const _kCookie = 'moodle_session';

  Future<void> init() async {
    final dir = await getApplicationDocumentsDirectory();
    _docsDir = dir.path;
    await rust.openDatabase(path: '${dir.path}/0spof.db');
    version = await rust.appVersion();
    try {
      final a = await DeviceInfoPlugin().androidInfo;
      deviceName = '${a.manufacturer} ${a.model}'.trim();
    } catch (_) {}
    await _restoreSession();
    await refresh();
    WidgetsBinding.instance.addObserver(this);
    _timer = Timer.periodic(const Duration(minutes: 1), (_) => syncPc(silent: true));
    unawaited(syncPc(silent: true));
    unawaited(checkUpdate());
  }

  Future<void> checkUpdate() async {
    update = await AppUpdate.check();
    notifyListeners();
  }

  /// Baixa o APK (conferindo o sha256) e abre o instalador do sistema.
  void installUpdate() {
    final u = update;
    if (u == null || updateProgress != null) return;
    updateProgress = 0;
    updateError = null;
    notifyListeners();
    u.install().listen(
      (e) {
        if (e.status == OtaStatus.DOWNLOADING) {
          updateProgress = int.tryParse(e.value ?? '') ?? updateProgress;
        } else if (e.status == OtaStatus.INSTALLING || e.status == OtaStatus.INSTALLATION_DONE) {
          updateProgress = null;
        } else {
          updateProgress = null;
          updateError = 'Falha na atualização (${e.status.name})';
        }
        notifyListeners();
      },
      onError: (Object e) {
        updateProgress = null;
        updateError = '$e';
        notifyListeners();
      },
    );
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

  /// O cookie fica cifrado no Keystore do Android; o Rust só o mantém em memória.
  Future<void> _restoreSession() async {
    try {
      final url = await _secure.read(key: _kBaseUrl);
      final cookie = await _secure.read(key: _kCookie);
      if (url != null && cookie != null) {
        await rust.saveSession(baseUrl: url, sessionCookie: cookie);
      }
    } catch (_) {
      // Keystore indisponível: o usuário só precisa entrar de novo.
    }
  }

  Future<String> savedMoodleUrl() async =>
      (await _secure.read(key: _kBaseUrl)) ?? session.baseUrl ?? defaultMoodleUrl;

  /// Chamado pelo WebView de login ao capturar o cookie `MoodleSession`. Já dispara o crawl.
  Future<void> loginMoodle(String baseUrl, String cookie) async {
    final info = SessionInfo.parse(await rust.saveSession(baseUrl: baseUrl, sessionCookie: cookie));
    await _secure.write(key: _kBaseUrl, value: info.baseUrl ?? baseUrl);
    await _secure.write(key: _kCookie, value: cookie);
    await refresh();
    unawaited(syncMoodle());
  }

  Future<void> logoutMoodle() async {
    await rust.logout();
    await _secure.delete(key: _kCookie);
    await NativeCookies.clear();
    lastSyncMessage = null;
    await refresh();
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

  late String _docsDir;

  /// Copia o arquivo escolhido para a pasta do app (o seletor devolve um cache temporário),
  /// calcula o SHA-256 e pede o carimbo RFC 3161 (se `fetchTsa`).
  Future<EnqueueResult> enqueueSubmission(int assignId, String pickedPath, {bool fetchTsa = true}) async {
    final dir = Directory('$_docsDir/submissions');
    await dir.create(recursive: true);
    final name = pickedPath.split(RegExp(r'[\\/]')).last;
    final stamp = DateTime.now().millisecondsSinceEpoch;
    final dest = await File(pickedPath).copy('${dir.path}/${stamp}_$name');
    final r = EnqueueResult.parse(
        await rust.enqueueSubmission(assignId: assignId, filePath: dest.path, fetchTsa: fetchTsa));
    await refresh();
    return r;
  }

  Future<Proof> proof(int outboxId) async => Proof.parse(await rust.submissionProof(outboxId: outboxId));

  /// Baixa o material da atividade para o celular e devolve o caminho local.
  Future<String> downloadMedia(int activityId, {bool allowVideo = false}) async {
    final path = await rust.downloadActivityMedia(activityId: activityId, allowVideo: allowVideo);
    await refresh();
    return path;
  }

  Future<String> exportData() => rust.exportData();

  /// Devolve um resumo do que foi importado.
  Future<String> importData(String json) async {
    final j = jsonDecode(await rust.importData(jsonContent: json));
    await refresh();
    return '${j['courses_imported']} disciplinas, ${j['activities_imported']} atividades, '
        '${j['outbox_items_imported']} entregas';
  }

  Future<CourseExportPreview> courseExportPreview(int courseId) async =>
      CourseExportPreview.parse(await rust.courseExportPreview(courseId: courseId));

  Future<Uint8List> exportCoursePackage({
    required int courseId,
    required List<int> activityIds,
    List<int> includeSectionIds = const [],
    bool includeFiles = true,
  }) =>
      rust.exportCoursePackage(
        selectionJson: jsonEncode({
          'course_id': courseId,
          'activity_ids': activityIds,
          'include_section_ids': includeSectionIds,
          'include_files': includeFiles,
        }),
      );

  /// Importa `.json` ou `.zip` a partir do caminho local e devolve um resumo.
  Future<String> importPackageFile(String path) async {
    final j = jsonDecode(await rust.importPackageFile(path: path));
    await refresh();
    final files = (j['files_imported'] as num?)?.toInt() ?? 0;
    final filesPart = files > 0 ? ', $files arquivos offline' : '';
    return '${j['courses_imported']} disciplinas, ${j['activities_imported']} atividades$filesPart, '
        '${j['outbox_items_imported']} entregas';
  }

  Future<void> wipeAllData() async {
    await rust.wipeAllData();
    await refresh();
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
