import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';

import 'models.dart';
import 'src/rust/api/spof.dart' as rust;

/// Estado do app: lê do banco local (via bridge) e notifica as telas.
class Store extends ChangeNotifier {
  SystemStatus? status;
  List<Course> courses = [];
  List<Deadline> deadlines = [];
  List<OutboxItem> outbox = [];
  SessionInfo session = SessionInfo.fromJson({'is_authenticated': false, 'base_url': null});
  String version = '';
  bool syncing = false;
  String? lastError;
  String? lastSyncMessage;

  Future<void> init() async {
    final dir = await getApplicationDocumentsDirectory();
    await rust.openDatabase(path: '${dir.path}/0spof.db');
    version = await rust.appVersion();
    await refresh();
  }

  Future<void> refresh() async {
    try {
      status = SystemStatus.parse(await rust.systemStatus());
      courses = Course.parseList(await rust.courses());
      deadlines = Deadline.parseList(await rust.upcomingDeadlines());
      outbox = OutboxItem.parseList(await rust.outboxItems());
      session = SessionInfo.parse(await rust.sessionInfo());
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
