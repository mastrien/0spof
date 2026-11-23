import 'package:flutter/material.dart';
import 'models.dart';
import 'activity_actions.dart';
import 'course_export_page.dart';
import 'data_actions.dart';
import 'login_page.dart';
import 'pair_page.dart';
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

  /// Escolhe a tarefa (entre as com prazo) e depois o arquivo.
  Future<void> _newSubmission(BuildContext context, Store s) async {
    final assigns = s.deadlines.where((d) => d.type == 'assign').toList();
    if (assigns.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Nenhuma tarefa com prazo. Abra a tarefa pela disciplina.')));
      return;
    }
    final chosen = await showModalBottomSheet<int>(
      context: context,
      showDragHandle: true,
      builder: (sheet) => SafeArea(
        child: ListView(
          shrinkWrap: true,
          children: [
            for (final d in assigns)
              ListTile(
                leading: const Icon(Icons.assignment_outlined),
                title: Text(d.title, maxLines: 2, overflow: TextOverflow.ellipsis),
                subtitle: d.due == null ? null : Text(fmtDue(d.due!)),
                onTap: () => Navigator.pop(sheet, d.id),
              ),
          ],
        ),
      ),
    );
    if (chosen != null && context.mounted) await pickAndEnqueue(context, s, chosen);
  }

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
          floatingActionButton: _tab == 2
              ? FloatingActionButton.extended(
                  onPressed: () => _newSubmission(context, s),
                  icon: const Icon(Icons.add),
                  label: const Text('Nova entrega'),
                )
              : null,
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
              NavigationDestination(
                icon: Badge(isLabelVisible: s.update != null, smallSize: 8, child: const Icon(Icons.settings_outlined)),
                selectedIcon: const Icon(Icons.settings),
                label: 'Ajustes',
              ),
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
      onRefresh: () async {
        await store.syncPc();
        await store.refresh();
      },
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
                    onTap: () => showActivityActions(context, store, id: d.id, title: d.title, type: d.type, url: d.url),
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
      onRefresh: () async {
        await store.syncPc();
        await store.refresh();
      },
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
                      trailing: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          if (c.muted) const Icon(Icons.notifications_off_outlined),
                          IconButton(
                            tooltip: 'Exportar / compartilhar disciplina',
                            icon: const Icon(Icons.share_outlined),
                            onPressed: () => Navigator.push(
                              context,
                              MaterialPageRoute(
                                builder: (_) => CourseExportPage(store: store, course: c),
                              ),
                            ),
                          ),
                          if (!c.muted) const Icon(Icons.chevron_right),
                        ],
                      ),
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
  late Future<List<Section>> _future = widget.store.courseDetails(widget.course.id);
  late bool _muted = widget.course.muted;

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(
          title: Text(widget.course.shortname, overflow: TextOverflow.ellipsis),
          actions: [
            IconButton(
              tooltip: 'Exportar / compartilhar disciplina',
              icon: const Icon(Icons.share_outlined),
              onPressed: () => Navigator.push(
                context,
                MaterialPageRoute(
                  builder: (_) => CourseExportPage(store: widget.store, course: widget.course),
                ),
              ),
            ),
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
                            onTap: () async {
                              await showActivityActions(context, widget.store,
                                  id: a.id, title: a.title, type: a.type, url: a.url, localPath: a.localFilePath);
                              setState(() => _future = widget.store.courseDetails(widget.course.id));
                            },
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
      onRefresh: () async {
        await store.syncPc();
        await store.refresh();
      },
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
                    onTap: () => showProof(context, store, o.id),
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
    final up = store.update;
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        if (up != null) ...[
          Card(
            color: blue.withValues(alpha: 0.12),
            child: ListTile(
              leading: const Icon(Icons.system_update, color: blue),
              title: Text('Versão ${up.version} disponível'),
              subtitle: Text(store.updateError ??
                  (store.updateProgress != null ? 'Baixando… ${store.updateProgress}%' : 'Toque para baixar e instalar.')),
              onTap: store.updateProgress == null ? store.installUpdate : null,
            ),
          ),
          const SizedBox(height: 10),
        ],
        Card(
          child: ListTile(
            leading: Icon(store.session.authenticated ? Icons.link : Icons.link_off, color: store.session.authenticated ? Colors.green : muted),
            title: Text(store.session.authenticated ? 'Conectado ao Moodle' : 'Entrar no Moodle'),
            subtitle: Text(store.session.baseUrl ?? 'Login institucional (SUAP / Moodle) direto no celular.'),
            trailing: store.session.authenticated
                ? TextButton(onPressed: store.logoutMoodle, child: const Text('Sair'))
                : const Icon(Icons.login),
            onTap: store.session.authenticated
                ? null
                : () => Navigator.push(context, MaterialPageRoute(builder: (_) => LoginPage(store: store))),
          ),
        ),
        const SizedBox(height: 10),
        Card(
          child: ListTile(
            leading: Icon(store.peer != null ? Icons.computer : Icons.computer_outlined, color: store.peer != null ? Colors.green : muted),
            title: Text(store.peer != null ? 'Pareado com ${store.peer!.name.isEmpty ? store.peer!.host : store.peer!.name}' : 'Parear com o PC'),
            subtitle: Text(store.peer == null
                ? 'Leia o QR code do app desktop (Ajustes) na rede de casa.'
                : store.pcSyncError ??
                    (store.pcSyncing
                        ? 'Sincronizando…'
                        : store.pcLastSync == null
                            ? '${store.peer!.host}:${store.peer!.port}'
                            : 'Última sincronização: ${fmtDue(store.pcLastSync!).split(' · ').first}')),
            trailing: store.peer == null
                ? const Icon(Icons.qr_code_scanner)
                : PopupMenuButton<String>(
                    onSelected: (v) => v == 'sync' ? store.syncPc() : store.unpairPc(),
                    itemBuilder: (_) => const [
                      PopupMenuItem(value: 'sync', child: Text('Sincronizar agora')),
                      PopupMenuItem(value: 'unpair', child: Text('Desparear')),
                    ],
                  ),
            onTap: store.peer == null
                ? () => Navigator.push(context, MaterialPageRoute(builder: (_) => PairPage(store: store)))
                : null,
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
        const SizedBox(height: 10),
        DataCard(store: store),
        const SizedBox(height: 24),
        Center(child: Text('0spof ${store.version}', style: const TextStyle(color: muted))),
      ],
    );
  }
}
