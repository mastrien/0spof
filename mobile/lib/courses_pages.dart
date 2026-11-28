import 'dart:io';

import 'package:flutter/material.dart';

import 'activity_actions.dart';
import 'course_export_page.dart';
import 'data_actions.dart';
import 'format.dart';
import 'models.dart';
import 'screens.dart';
import 'store.dart';
import 'theme.dart';
import 'widgets.dart';

enum _CourseFilter { all, active, muted }

class CoursesPage extends StatefulWidget {
  final Store store;
  const CoursesPage({super.key, required this.store});
  @override
  State<CoursesPage> createState() => _CoursesPageState();
}

class _CoursesPageState extends State<CoursesPage> {
  String _query = '';
  _CourseFilter _filter = _CourseFilter.all;

  static const _labels = {
    _CourseFilter.all: 'Todas',
    _CourseFilter.active: 'Ativas',
    _CourseFilter.muted: 'Silenciadas',
  };

  @override
  Widget build(BuildContext context) {
    final store = widget.store;
    final all = store.courses;
    final q = _query.trim().toLowerCase();
    final items = all.where((c) {
      if (_filter == _CourseFilter.active && c.muted) return false;
      if (_filter == _CourseFilter.muted && !c.muted) return false;
      if (q.isEmpty) return true;
      return '${c.fullname} ${c.shortname}'.toLowerCase().contains(q);
    }).toList();

    final children = <Widget>[
      ScreenHeader(
        eyebrow: '${all.length} ${all.length == 1 ? 'disciplina salva' : 'disciplinas salvas'}',
        title: 'Disciplinas',
        trailing: HeaderIconButton(
          tooltip: 'Importar pacote',
          icon: Icons.file_upload_outlined,
          onPressed: () => importPackage(context, store),
        ),
      ),
      Padding(
        padding: const EdgeInsets.fromLTRB(16, 14, 16, 0),
        child: TextField(
          textInputAction: TextInputAction.search,
          decoration: const InputDecoration(
            hintText: 'Buscar por nome ou sigla…',
            prefixIcon: Icon(Icons.search, size: 20),
            constraints: BoxConstraints(minHeight: 48),
          ),
          onChanged: (v) => setState(() => _query = v),
        ),
      ),
      SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.fromLTRB(16, 4, 16, 6),
        child: Row(
          children: [
            for (final f in _CourseFilter.values) ...[
              FilterPill(
                label: _labels[f]!,
                selected: _filter == f,
                onTap: () => setState(() => _filter = f),
              ),
              const SizedBox(width: 8),
            ],
          ],
        ),
      ),
    ];

    if (all.isEmpty) {
      children.add(emptyState(Icons.school_outlined, 'Nenhuma disciplina no banco local.'));
    } else if (items.isEmpty) {
      children.add(emptyState(Icons.search_off, 'Nenhuma disciplina corresponde à busca.'));
    } else {
      for (final c in items) {
        children.add(_CourseCard(store: store, c: c));
      }
    }
    return refreshableList(store, children);
  }
}

class _CourseCard extends StatelessWidget {
  final Store store;
  final Course c;
  const _CourseCard({required this.store, required this.c});

  @override
  Widget build(BuildContext context) {
    final p = Pal.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 6, 16, 6),
      child: Material(
          // silenciada: fundo "mut" em vez de opacidade, para manter o contraste do texto
          color: c.muted ? p.mutBg : p.surface,
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16), side: BorderSide(color: p.border)),
          clipBehavior: Clip.antiAlias,
          child: InkWell(
            onTap: () => Navigator.push(context, MaterialPageRoute(builder: (_) => CourseDetailPage(store: store, course: c))),
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Eyebrow(c.shortname),
                            const SizedBox(height: 2),
                            Text(c.fullname,
                                maxLines: 2,
                                overflow: TextOverflow.ellipsis,
                                style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: p.ink)),
                          ],
                        ),
                      ),
                      if (c.muted) ...[const SizedBox(width: 8), const StatusChip('Silenciada', icon: Icons.notifications_off_outlined)],
                    ],
                  ),
                  const SizedBox(height: 8),
                  Text('${c.sections} seções · ${c.activities} atividades', style: TextStyle(fontSize: 12, color: p.muted)),
                ],
              ),
            ),
          ),
        ),
    );
  }
}

class _Detail {
  final List<Section> sections;
  final Map<int, int> sizes; // activity id -> bytes do arquivo baixado
  const _Detail(this.sections, this.sizes);
  int get totalBytes => sizes.values.fold(0, (a, b) => a + b);
}

class CourseDetailPage extends StatefulWidget {
  final Store store;
  final Course course;
  const CourseDetailPage({super.key, required this.store, required this.course});
  @override
  State<CourseDetailPage> createState() => _CourseDetailPageState();
}

class _CourseDetailPageState extends State<CourseDetailPage> {
  late Future<_Detail> _future = _load();
  late bool _muted = widget.course.muted;
  final Set<int> _expanded = {};
  bool _expandedInit = false;

  Future<_Detail> _load() async {
    final sections = await widget.store.courseDetails(widget.course.id);
    final sizes = <int, int>{};
    for (final s in sections) {
      for (final a in s.activities) {
        final path = a.localFilePath;
        if (path == null) continue;
        try {
          sizes[a.id] = await File(path).length();
        } catch (_) {
          // arquivo removido do disco: ignora o tamanho
        }
      }
    }
    return _Detail(sections, sizes);
  }

  Activity? _nextAssign(List<Section> sections) {
    final now = DateTime.now();
    Activity? best;
    for (final s in sections) {
      for (final a in s.activities) {
        if (a.type != 'assign' || a.submitted || a.due == null || !a.due!.isAfter(now)) continue;
        if (best == null || a.due!.isBefore(best.due!)) best = a;
      }
    }
    return best;
  }

  ({String label, Tone tone}) _kind(Activity a) {
    final mime = (a.mime ?? '').toLowerCase();
    final path = (a.localFilePath ?? a.url ?? '').toLowerCase();
    if (mime.contains('pdf') || path.endsWith('.pdf')) return (label: 'PDF', tone: Tone.err);
    if (mime.contains('html')) return (label: 'HTML', tone: Tone.info);
    if (mime.startsWith('image/')) return (label: 'IMG', tone: Tone.mut);
    if (mime.startsWith('video/')) return (label: 'VÍD', tone: Tone.warn);
    return switch (a.type) {
      'assign' => (label: 'TAR', tone: Tone.warn),
      'quiz' => (label: 'QUIZ', tone: Tone.warn),
      'forum' => (label: 'FÓR', tone: Tone.mut),
      'url' => (label: 'URL', tone: Tone.mut),
      'folder' => (label: 'PAS', tone: Tone.mut),
      'page' => (label: 'PÁG', tone: Tone.info),
      _ => (label: 'ARQ', tone: Tone.mut),
    };
  }

  Future<void> _open(Activity a) async {
    await showActivityActions(context, widget.store,
        id: a.id, title: a.title, type: a.type, url: a.url, localPath: a.localFilePath);
    if (mounted) setState(() => _future = _load());
  }

  Widget _row(BuildContext context, Activity a, _Detail d) {
    final p = Pal.of(context);
    final kind = _kind(a);
    final size = d.sizes[a.id];
    final sub = size != null
        ? formatFileSize(size)
        : a.due != null
            ? fmtDue(a.due!)
            : typeLabel(a.type);
    final Widget? chip = a.localFilePath != null
        ? const StatusChip('Offline', tone: Tone.ok)
        : a.submitted
            ? const StatusChip('Enviado', tone: Tone.ok)
            : (a.type == 'resource' || a.type == 'folder' || a.type == 'url')
                ? const StatusChip('Só online')
                : null;
    return InkWell(
      onTap: () => _open(a),
      child: Container(
        constraints: const BoxConstraints(minHeight: 56),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
        child: Row(
          children: [
            Container(
              width: 36,
              height: 36,
              alignment: Alignment.center,
              decoration: BoxDecoration(color: p.bgOf(kind.tone), borderRadius: BorderRadius.circular(9)),
              child: Text(kind.label,
                  style: TextStyle(fontSize: 10, fontWeight: FontWeight.w700, color: p.fgOf(kind.tone))),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(a.title,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, height: 1.3, color: p.ink)),
                  Text(sub, style: TextStyle(fontSize: 12, color: p.muted)),
                ],
              ),
            ),
            if (chip != null) ...[const SizedBox(width: 8), chip],
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final p = Pal.of(context);
    final course = widget.course;
    return Scaffold(
      appBar: AppBar(
        toolbarHeight: 64,
        titleSpacing: 0,
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Eyebrow(course.shortname),
            Text(course.fullname,
                maxLines: 1, overflow: TextOverflow.ellipsis, style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w700)),
          ],
        ),
        actions: [
          IconButton(
            tooltip: 'Exportar ou compartilhar',
            icon: const Icon(Icons.ios_share),
            onPressed: () => Navigator.push(
              context,
              MaterialPageRoute(builder: (_) => CourseExportPage(store: widget.store, course: course)),
            ),
          ),
          IconButton(
            tooltip: _muted ? 'Reativar disciplina' : 'Silenciar disciplina',
            icon: Icon(_muted ? Icons.notifications_off : Icons.notifications_none),
            onPressed: () async {
              await widget.store.setMuted(course.id, !_muted);
              setState(() => _muted = !_muted);
            },
          ),
        ],
      ),
      body: FutureBuilder<_Detail>(
        future: _future,
        builder: (context, snap) {
          if (snap.hasError) return ListView(children: [emptyState(Icons.error_outline, '${snap.error}')]);
          if (!snap.hasData) return const Center(child: CircularProgressIndicator());
          final detail = snap.data!;
          final sections = detail.sections.where((s) => s.activities.isNotEmpty || (s.summary ?? '').isNotEmpty).toList();
          if (sections.isEmpty) return ListView(children: [emptyState(Icons.inbox_outlined, 'Sem conteúdo nesta disciplina.')]);
          if (!_expandedInit) {
            _expanded.add(sections.first.id);
            _expandedInit = true;
          }
          final next = _nextAssign(sections);
          final hasOffline = detail.sizes.isNotEmpty ||
              sections.any((s) => s.activities.any((a) => a.localFilePath != null));
          return ListView(
            padding: const EdgeInsets.fromLTRB(16, 14, 16, 24),
            children: [
              if (next != null)
                Material(
                  color: p.warnCardBg,
                  shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(16), side: BorderSide(color: p.warnCardBorder)),
                  clipBehavior: Clip.antiAlias,
                  child: InkWell(
                    onTap: () => _open(next),
                    child: Padding(
                      padding: const EdgeInsets.all(14),
                      child: Row(
                        children: [
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Eyebrow('Vence ${fmtShortDate(next.due!)} · '
                                    '${next.due!.hour.toString().padLeft(2, '0')}:${next.due!.minute.toString().padLeft(2, '0')}',
                                    color: p.warnFg),
                                const SizedBox(height: 2),
                                Text(next.title,
                                    maxLines: 3,
                                    overflow: TextOverflow.ellipsis,
                                    style: TextStyle(fontSize: 15, fontWeight: FontWeight.w700, height: 1.3, color: p.ink)),
                              ],
                            ),
                          ),
                          if (queuedFor(widget.store, next.id) != null) ...[
                            const SizedBox(width: 8),
                            const StatusChip('Na fila', tone: Tone.info),
                          ],
                        ],
                      ),
                    ),
                  ),
                ),
              if (hasOffline)
                Padding(
                  padding: const EdgeInsets.only(top: 10),
                  child: Wrap(
                    spacing: 8,
                    runSpacing: 6,
                    children: [
                      const StatusChip('Disponível offline', tone: Tone.ok),
                      if (detail.totalBytes > 0) StatusChip('${formatFileSize(detail.totalBytes)} salvos'),
                    ],
                  ),
                ),
              const SizedBox(height: 14),
              for (var i = 0; i < sections.length; i++) _section(context, i, sections[i], detail),
            ],
          );
        },
      ),
    );
  }

  Widget _section(BuildContext context, int index, Section s, _Detail detail) {
    final p = Pal.of(context);
    final open = _expanded.contains(s.id);
    return SpofCard(
      margin: const EdgeInsets.only(bottom: 12),
      child: Column(
        children: [
          Semantics(
            button: true,
            expanded: open,
            child: InkWell(
              onTap: () => setState(() => open ? _expanded.remove(s.id) : _expanded.add(s.id)),
              child: Container(
                constraints: const BoxConstraints(minHeight: 52),
                padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                child: Row(
                  children: [
                    Text((index + 1).toString().padLeft(2, '0'),
                        style: TextStyle(fontSize: 12, fontWeight: FontWeight.w600, color: p.muted)),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Text(s.name,
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(fontSize: 15, fontWeight: FontWeight.w700, color: p.ink)),
                    ),
                    Icon(open ? Icons.expand_less : Icons.expand_more, color: p.muted),
                  ],
                ),
              ),
            ),
          ),
          if (open)
            for (final a in s.activities) ...[
              Divider(height: 1, thickness: 1, color: p.divider),
              _row(context, a, detail),
            ],
        ],
      ),
    );
  }
}
