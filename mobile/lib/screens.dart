import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import 'models.dart';
import 'store.dart';
import 'theme.dart';

String fmtDue(DateTime d) {
  final now = DateTime.now();
  final days = DateTime(d.year, d.month, d.day).difference(DateTime(now.year, now.month, now.day)).inDays;
  final hm = '${d.hour.toString().padLeft(2, '0')}:${d.minute.toString().padLeft(2, '0')}';
  final date = '${d.day.toString().padLeft(2, '0')}/${d.month.toString().padLeft(2, '0')}';
  final rel = days == 0
      ? 'hoje'
      : days == 1
          ? 'amanhã'
          : days > 1
              ? 'em $days dias'
              : 'passou há ${-days} d';
  return '$date $hm · $rel';
}

IconData typeIcon(String t) => switch (t) {
      'assign' => Icons.assignment_outlined,
      'resource' => Icons.description_outlined,
      'folder' => Icons.folder_outlined,
      'url' => Icons.link,
      'forum' => Icons.forum_outlined,
      'quiz' => Icons.quiz_outlined,
      _ => Icons.extension_outlined,
    };

class HomeShell extends StatefulWidget {
  final Store store;
  const HomeShell({super.key, required this.store});
  @override
  State<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends State<HomeShell> {
  int _tab = 0;

  static const _titles = ['Prazos', 'Disciplinas', 'Entregas', 'Ajustes'];

  @override
  Widget build(BuildContext context) {
    final s = widget.store;
    return ListenableBuilder(
      listenable: s,
      builder: (context, _) {
        final pages = [
          DeadlinesPage(store: s),
          CoursesPage(store: s),
          OutboxPage(store: s),
          SettingsPage(store: s),
        ];
        return Scaffold(
          appBar: AppBar(
            title: Text(_titles[_tab], style: const TextStyle(fontWeight: FontWeight.w700)),
            bottom: s.syncing ? const PreferredSize(preferredSize: Size.fromHeight(2), child: LinearProgressIndicator(minHeight: 2)) : null,
          ),
          body: SafeArea(child: pages[_tab]),
          bottomNavigationBar: NavigationBar(
            selectedIndex: _tab,
            onDestinationSelected: (i) => setState(() => _tab = i),
            destinations: [
              const NavigationDestination(icon: Icon(Icons.event_outlined), selectedIcon: Icon(Icons.event), label: 'Prazos'),
              const NavigationDestination(icon: Icon(Icons.school_outlined), selectedIcon: Icon(Icons.school), label: 'Disciplinas'),
              NavigationDestination(
                icon: Badge(isLabelVisible: s.outbox.isNotEmpty, label: Text('${s.outbox.length}'), child: const Icon(Icons.outbox_outlined)),
                selectedIcon: const Icon(Icons.outbox),
                label: 'Entregas',
              ),
              const NavigationDestination(icon: Icon(Icons.settings_outlined), selectedIcon: Icon(Icons.settings), label: 'Ajustes'),
            ],
          ),
        );
      },
    );
  }
}

Widget _empty(IconData icon, String text) => ListView(
      physics: const AlwaysScrollableScrollPhysics(),
      children: [
        const SizedBox(height: 120),
        Icon(icon, size: 48, color: muted),
        const SizedBox(height: 12),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 32),
          child: Text(text, textAlign: TextAlign.center, style: const TextStyle(color: muted)),
        ),
      ],
    );

class DeadlinesPage extends StatelessWidget {
  final Store store;
  const DeadlinesPage({super.key, required this.store});

  @override
  Widget build(BuildContext context) {
    final items = store.deadlines;
    return RefreshIndicator(
      onRefresh: store.refresh,
      child: items.isEmpty
          ? _empty(Icons.event_available, 'Nenhum prazo pela frente.\nSincronize para trazer as atividades.')
          : ListView.separated(
              padding: const EdgeInsets.all(16),
              itemCount: items.length,
              separatorBuilder: (_, _) => const SizedBox(height: 10),
              itemBuilder: (_, i) {
                final d = items[i];
                final late = d.due != null && d.due!.isBefore(DateTime.now());
                return Card(
                  child: ListTile(
                    leading: Icon(typeIcon(d.type), color: late && !d.submitted ? Colors.redAccent : blue),
                    title: Text(d.title, maxLines: 2, overflow: TextOverflow.ellipsis),
                    subtitle: d.due == null ? null : Text(fmtDue(d.due!)),
                    trailing: d.submitted ? const Icon(Icons.check_circle, color: Colors.green) : null,
                    onTap: d.url == null ? null : () => launchUrl(Uri.parse(d.url!), mode: LaunchMode.externalApplication),
                  ),
                );
              },
            ),
    );
  }
}

class CoursesPage extends StatelessWidget {
  final Store store;
  const CoursesPage({super.key, required this.store});

  @override
  Widget build(BuildContext context) {
    final items = store.courses;
    return RefreshIndicator(
      onRefresh: store.refresh,
      child: items.isEmpty
          ? _empty(Icons.school_outlined, 'Nenhuma disciplina no banco local.')
          : ListView.separated(
              padding: const EdgeInsets.all(16),
              itemCount: items.length,
              separatorBuilder: (_, _) => const SizedBox(height: 10),
              itemBuilder: (_, i) {
                final c = items[i];
                return Opacity(
                  opacity: c.muted ? 0.5 : 1,
                  child: Card(
                    child: ListTile(
                      title: Text(c.fullname, maxLines: 2, overflow: TextOverflow.ellipsis),
                      subtitle: Text('${c.sections} seções · ${c.activities} atividades'),
                      trailing: c.muted ? const Icon(Icons.notifications_off_outlined) : const Icon(Icons.chevron_right),
                      onTap: () => Navigator.push(context, MaterialPageRoute(builder: (_) => CourseDetailPage(store: store, course: c))),
                    ),
                  ),
                );
              },
            ),
    );
  }
}

class CourseDetailPage extends StatefulWidget {
  final Store store;
  final Course course;
  const CourseDetailPage({super.key, required this.store, required this.course});
  @override
  State<CourseDetailPage> createState() => _CourseDetailPageState();
}

class _CourseDetailPageState extends State<CourseDetailPage> {
  late final Future<List<Section>> _future = widget.store.courseDetails(widget.course.id);
  late bool _muted = widget.course.muted;

  Future<void> _open(Activity a) async {
    final target = a.localFilePath ?? a.url;
    if (target == null) return;
    final uri = a.localFilePath != null ? Uri.file(a.localFilePath!) : Uri.parse(a.url!);
    final ok = await launchUrl(uri, mode: LaunchMode.externalApplication);
    if (!ok && mounted) {
      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Não foi possível abrir.')));
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(
          title: Text(widget.course.shortname, overflow: TextOverflow.ellipsis),
          actions: [
            IconButton(
              tooltip: _muted ? 'Reativar disciplina' : 'Silenciar disciplina',
              icon: Icon(_muted ? Icons.notifications_off : Icons.notifications_none),
              onPressed: () async {
                await widget.store.setMuted(widget.course.id, !_muted);
                setState(() => _muted = !_muted);
              },
            ),
          ],
        ),
        body: FutureBuilder<List<Section>>(
          future: _future,
          builder: (context, snap) {
            if (snap.hasError) return _empty(Icons.error_outline, '${snap.error}');
            if (!snap.hasData) return const Center(child: CircularProgressIndicator());
            final sections = snap.data!.where((s) => s.activities.isNotEmpty || (s.summary ?? '').isNotEmpty).toList();
            if (sections.isEmpty) return _empty(Icons.inbox_outlined, 'Sem conteúdo nesta disciplina.');
            return ListView(
              padding: const EdgeInsets.all(16),
              children: [
                for (final s in sections) ...[
                  Padding(
                    padding: const EdgeInsets.fromLTRB(4, 12, 4, 8),
                    child: Text(s.name, style: Theme.of(context).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700, color: muted)),
                  ),
                  Card(
                    child: Column(
                      children: [
                        for (final a in s.activities)
                          ListTile(
                            leading: Icon(typeIcon(a.type), color: blue),
                            title: Text(a.title, maxLines: 2, overflow: TextOverflow.ellipsis),
                            subtitle: a.due != null ? Text(fmtDue(a.due!)) : null,
                            trailing: a.localFilePath != null
                                ? const Icon(Icons.download_done, color: Colors.green)
                                : a.submitted
                                    ? const Icon(Icons.check_circle, color: Colors.green)
                                    : null,
                            onTap: () => _open(a),
                          ),
                      ],
                    ),
                  ),
                ],
              ],
            );
          },
        ),
      );
}

class OutboxPage extends StatelessWidget {
  final Store store;
  const OutboxPage({super.key, required this.store});

  @override
  Widget build(BuildContext context) {
    final items = store.outbox;
    return RefreshIndicator(
      onRefresh: store.refresh,
      child: items.isEmpty
          ? _empty(Icons.outbox_outlined, 'Nenhuma entrega na fila.')
          : ListView.separated(
              padding: const EdgeInsets.all(16),
              itemCount: items.length,
              separatorBuilder: (_, _) => const SizedBox(height: 10),
              itemBuilder: (_, i) {
                final o = items[i];
                return Card(
                  child: ListTile(
                    leading: Icon(o.hasToken ? Icons.verified_outlined : Icons.schedule, color: o.hasToken ? Colors.green : muted),
                    title: Text(o.filename, overflow: TextOverflow.ellipsis),
                    subtitle: Text('${o.status} · tentativas: ${o.retries}\nSHA-256 ${o.hash.substring(0, o.hash.length.clamp(0, 12))}…'),
                    isThreeLine: true,
                  ),
                );
              },
            ),
    );
  }
}

class SettingsPage extends StatelessWidget {
  final Store store;
  const SettingsPage({super.key, required this.store});

  @override
  Widget build(BuildContext context) {
    final s = store.status;
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Card(
          child: ListTile(
            leading: Icon(store.session.authenticated ? Icons.link : Icons.link_off, color: store.session.authenticated ? Colors.green : muted),
            title: Text(store.session.authenticated ? 'Conectado ao Moodle' : 'Sem sessão no Moodle'),
            subtitle: Text(store.session.baseUrl ?? 'O login próprio e o pareamento com o PC chegam nas próximas fases.'),
          ),
        ),
        const SizedBox(height: 10),
        if (s != null)
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Text('${s.courses} disciplinas (${s.mutedCourses} silenciadas) · ${s.deadlines} prazos · ${s.outbox} na fila'),
            ),
          ),
        const SizedBox(height: 10),
        Card(
          child: ListTile(
            leading: const Icon(Icons.sync),
            title: const Text('Sincronizar com o Moodle'),
            subtitle: Text(store.lastError ?? store.lastSyncMessage ?? 'Requer sessão ativa'),
            enabled: store.session.authenticated && !store.syncing,
            onTap: store.syncMoodle,
          ),
        ),
        const SizedBox(height: 24),
        Center(child: Text('0spof ${store.version}', style: const TextStyle(color: muted))),
      ],
    );
  }
}
