import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart' show setEquals;
import 'package:flutter/material.dart';

import 'models.dart';
import 'store.dart';
import 'theme.dart';
import 'widgets.dart';

enum ExportPreset { materials, pdfs, all, none }

class ExportSelectionStats {
  final int selectedCount;
  final int totalCount;
  final int pdfCount;
  final int totalBytes;

  const ExportSelectionStats({
    required this.selectedCount,
    required this.totalCount,
    required this.pdfCount,
    required this.totalBytes,
  });
}

String formatFileSize(int bytes) {
  if (bytes <= 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  var value = bytes.toDouble();
  var idx = 0;
  while (value >= 1024 && idx < units.length - 1) {
    value /= 1024;
    idx++;
  }
  final formatted =
      idx == 0 ? '${value.round()}' : value.toStringAsFixed(value < 10 ? 1 : 0).replaceAll('.', ',');
  return '$formatted ${units[idx]}';
}

Set<int> defaultSelectedIds(CourseExportPreview preview) => {
      for (final sec in preview.sections)
        for (final item in sec.items)
          if (item.selectedByDefault) item.id,
    };

Set<int> applyExportPreset(CourseExportPreview preview, ExportPreset preset) {
  final items = [for (final s in preview.sections) ...s.items];
  return switch (preset) {
    ExportPreset.materials => {for (final i in items) if (i.isMaterial) i.id},
    ExportPreset.pdfs => {for (final i in items) if (i.isPdf) i.id},
    ExportPreset.all => {for (final i in items) i.id},
    ExportPreset.none => <int>{},
  };
}

ExportSelectionStats computeExportStats(
  CourseExportPreview preview,
  Set<int> selectedIds, {
  bool includeFiles = true,
}) {
  var totalCount = 0;
  var selectedCount = 0;
  var pdfCount = 0;
  var totalBytes = 0;

  for (final sec in preview.sections) {
    for (final item in sec.items) {
      totalCount++;
      if (!selectedIds.contains(item.id)) continue;
      selectedCount++;
      if (item.isPdf) pdfCount++;
      if (includeFiles && item.hasLocalFile) {
        totalBytes += item.fileSizeBytes ?? 0;
      }
    }
  }

  return ExportSelectionStats(
    selectedCount: selectedCount,
    totalCount: totalCount,
    pdfCount: pdfCount,
    totalBytes: totalBytes,
  );
}

class CourseExportPage extends StatefulWidget {
  final Store store;
  final Course course;

  const CourseExportPage({super.key, required this.store, required this.course});

  @override
  State<CourseExportPage> createState() => _CourseExportPageState();
}

class _CourseExportPageState extends State<CourseExportPage> {
  CourseExportPreview? _preview;
  Object? _loadError;
  Set<int> _selected = <int>{};
  String _query = '';
  bool _includeFiles = true;
  bool _exporting = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final preview = await widget.store.courseExportPreview(widget.course.id);
      if (!mounted) return;
      setState(() {
        _preview = preview;
        _selected = defaultSelectedIds(preview);
        _loadError = null;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() => _loadError = e);
    }
  }

  bool _matchesQuery(ExportItemPreview item) {
    final q = _query.trim().toLowerCase();
    if (q.isEmpty) return true;
    final hay = '${item.title} ${item.activityType} ${item.fileName ?? ''} '
            '${item.isPdf ? 'pdf' : ''} ${item.isMaterial ? 'material' : 'atividade'}'
        .toLowerCase();
    return hay.contains(q);
  }

  String _activityTypeLabel(String type) => switch (type) {
        'assign' => 'Tarefa',
        'resource' => 'Arquivo',
        'folder' => 'Pasta',
        'url' => 'Link',
        'forum' => 'Fórum',
        'quiz' => 'Questionário',
        'page' => 'Página',
        _ => type,
      };

  Future<void> _export(CourseExportPreview preview) async {
    if (_selected.isEmpty || _exporting) return;
    setState(() => _exporting = true);
    try {
      final bytes = await widget.store.exportCoursePackage(
        courseId: preview.courseId,
        activityIds: _selected.toList(),
        includeFiles: _includeFiles,
      );
      final saved = await FilePicker.saveFile(
        fileName: preview.suggestedFilename,
        bytes: bytes,
        mimeType: 'application/zip',
        dialogTitle: 'Exportar disciplina',
      );
      if (!mounted) return;
      if (saved != null) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Pacote "${preview.suggestedFilename}" exportado.')),
        );
        Navigator.pop(context);
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Falha ao exportar: $e')),
        );
      }
    } finally {
      if (mounted) setState(() => _exporting = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final preview = _preview;
    return Scaffold(
      appBar: AppBar(
        toolbarHeight: 64,
        titleSpacing: 0,
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Eyebrow('Compartilhar disciplina'),
            Text(
              'Exportar · ${widget.course.shortname}',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w700),
            ),
          ],
        ),
      ),
      body: _loadError != null
          ? Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Text('Erro ao carregar itens: $_loadError', textAlign: TextAlign.center),
              ),
            )
          : preview == null
              ? const Center(child: CircularProgressIndicator())
              : _buildContent(context, preview),
    );
  }

  Widget _buildContent(BuildContext context, CourseExportPreview preview) {
    final p = Pal.of(context);
    final allItems = [for (final s in preview.sections) ...s.items];
    final nonMaterials = allItems.where((i) => !i.isMaterial).toList();
    final selectedNonMaterials = nonMaterials.where((i) => _selected.contains(i.id)).length;
    final activitiesOn = nonMaterials.isNotEmpty && selectedNonMaterials == nonMaterials.length;

    final stats = computeExportStats(preview, _selected, includeFiles: _includeFiles);
    final visibleSections = [
      for (final sec in preview.sections)
        (
          section: sec,
          items: sec.items.where(_matchesQuery).toList(),
        )
    ].where((entry) => entry.items.isNotEmpty).toList();

    bool isPreset(ExportPreset preset) => setEquals(_selected, applyExportPreset(preview, preset));

    final detail = [
      if (stats.pdfCount > 0) '${stats.pdfCount} PDF${stats.pdfCount > 1 ? 's' : ''}',
      if (stats.totalBytes > 0) formatFileSize(stats.totalBytes),
    ].join(' · ');

    return Column(
      children: [
        Expanded(
          child: ListView(
            padding: const EdgeInsets.only(bottom: 16),
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      preview.fullname,
                      style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w700, height: 1.3),
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 8),
                    TextField(
                      textInputAction: TextInputAction.search,
                      decoration: const InputDecoration(
                        hintText: 'Filtrar por título, PDF ou tipo…',
                        prefixIcon: Icon(Icons.search, size: 20),
                        constraints: BoxConstraints(minHeight: 48),
                      ),
                      onChanged: (v) => setState(() => _query = v),
                    ),
                  ],
                ),
              ),
              SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                padding: const EdgeInsets.fromLTRB(16, 4, 16, 4),
                child: Row(
                  children: [
                    FilterPill(
                      label: 'Materiais',
                      selected: isPreset(ExportPreset.materials),
                      onTap: () => setState(() => _selected = applyExportPreset(preview, ExportPreset.materials)),
                    ),
                    const SizedBox(width: 8),
                    FilterPill(
                      label: 'Só PDFs',
                      selected: isPreset(ExportPreset.pdfs),
                      onTap: () => setState(() => _selected = applyExportPreset(preview, ExportPreset.pdfs)),
                    ),
                    const SizedBox(width: 8),
                    FilterPill(
                      label: 'Incluir atividades',
                      selected: activitiesOn,
                      onTap: nonMaterials.isEmpty
                          ? () {}
                          : () => setState(() {
                                if (activitiesOn) {
                                  _selected.removeAll(nonMaterials.map((i) => i.id));
                                } else {
                                  _selected.addAll(nonMaterials.map((i) => i.id));
                                }
                              }),
                    ),
                    const SizedBox(width: 8),
                    FilterPill(
                      label: 'Tudo',
                      selected: isPreset(ExportPreset.all),
                      onTap: () => setState(() => _selected = applyExportPreset(preview, ExportPreset.all)),
                    ),
                    const SizedBox(width: 8),
                    FilterPill(
                      label: 'Limpar',
                      selected: false,
                      onTap: () => setState(() => _selected = applyExportPreset(preview, ExportPreset.none)),
                    ),
                  ],
                ),
              ),
              SpofCard(
                margin: const EdgeInsets.fromLTRB(16, 8, 16, 0),
                child: SwitchListTile(
                  contentPadding: const EdgeInsets.symmetric(horizontal: 14),
                  title: const Text('Anexar arquivos baixados', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600)),
                  subtitle: Text('PDFs e cópias offline', style: TextStyle(fontSize: 12, color: p.muted)),
                  value: _includeFiles,
                  onChanged: (v) => setState(() => _includeFiles = v),
                ),
              ),
              const SizedBox(height: 12),
              if (visibleSections.isEmpty)
                Padding(
                  padding: const EdgeInsets.all(32),
                  child: Center(child: Text('Nenhum item corresponde ao filtro.', style: TextStyle(color: p.muted))),
                )
              else
                for (var idx = 0; idx < visibleSections.length; idx++) _buildSection(context, visibleSections[idx].section, visibleSections[idx].items),
            ],
          ),
        ),
        Container(
          decoration: BoxDecoration(color: p.surface, border: Border(top: BorderSide(color: p.border))),
          child: SafeArea(
            top: false,
            child: Padding(
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
              child: Row(
                children: [
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          '${stats.selectedCount} de ${stats.totalCount} itens',
                          style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
                        ),
                        if (detail.isNotEmpty)
                          Text(detail, style: TextStyle(fontWeight: FontWeight.w500, fontSize: 13, color: p.muted)),
                      ],
                    ),
                  ),
                  const SizedBox(width: 12),
                  FilledButton.icon(
                    onPressed: _selected.isEmpty || _exporting ? null : () => _export(preview),
                    icon: _exporting
                        ? SizedBox(
                            width: 16,
                            height: 16,
                            child: CircularProgressIndicator(strokeWidth: 2, color: Theme.of(context).colorScheme.onPrimary),
                          )
                        : const Icon(Icons.archive_outlined, size: 18),
                    label: Text(_exporting ? 'Gerando…' : 'Exportar (.zip)'),
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildSection(BuildContext context, ExportSectionPreview sec, List<ExportItemPreview> items) {
    final p = Pal.of(context);
    final selectedInSec = items.where((i) => _selected.contains(i.id)).length;
    final bool? secValue = selectedInSec == 0
        ? false
        : selectedInSec == items.length
            ? true
            : null;
    return SpofCard(
      margin: const EdgeInsets.fromLTRB(16, 0, 16, 12),
      child: Column(
        children: [
          CheckboxListTile(
            tristate: true,
            tileColor: p.mutBg,
            controlAffinity: ListTileControlAffinity.leading,
            contentPadding: const EdgeInsets.symmetric(horizontal: 12),
            value: secValue,
            title: Text(
              sec.name,
              style: const TextStyle(fontWeight: FontWeight.w700, fontSize: 14),
            ),
            secondary: Text(
              '${items.length} ${items.length == 1 ? 'item' : 'itens'}',
              style: TextStyle(fontSize: 12, color: p.muted),
            ),
            onChanged: (_) => setState(() {
              if (selectedInSec == items.length) {
                _selected.removeAll(items.map((i) => i.id));
              } else {
                _selected.addAll(items.map((i) => i.id));
              }
            }),
          ),
          for (final item in items) ...[
            Divider(height: 1, thickness: 1, color: p.divider),
            _buildItemTile(context, item),
          ],
        ],
      ),
    );
  }

  Widget _buildItemTile(BuildContext context, ExportItemPreview item) {
    final p = Pal.of(context);
    final checked = _selected.contains(item.id);
    final fileMeta = item.hasLocalFile
        ? '${item.fileName ?? 'Offline'} ·${formatFileSize(item.fileSizeBytes ?? 0)}'
        : item.url != null
            ? 'apenas link'
            : 'somente registro';
    final typeText = _activityTypeLabel(item.activityType);

    return CheckboxListTile(
      controlAffinity: ListTileControlAffinity.leading,
      contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 2),
      value: checked,
      onChanged: (v) => setState(() {
        if (v == true) {
          _selected.add(item.id);
        } else {
          _selected.remove(item.id);
        }
      }),
      title: Text.rich(
        TextSpan(
          children: [
            if (item.isPdf)
              WidgetSpan(
                alignment: PlaceholderAlignment.middle,
                child: Container(
                  margin: const EdgeInsets.only(right: 6),
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(color: p.errBg, borderRadius: BorderRadius.circular(5)),
                  child: Text(
                    'PDF',
                    style: TextStyle(color: p.errFg, fontWeight: FontWeight.w700, fontSize: 10.5),
                  ),
                ),
              ),
            TextSpan(text: item.title),
          ],
        ),
        maxLines: 3,
        overflow: TextOverflow.ellipsis,
        style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, height: 1.3, color: checked ? p.ink : p.muted),
      ),
      subtitle: Text.rich(
        TextSpan(
          style: TextStyle(fontSize: 12, color: p.muted),
          children: [
            TextSpan(
              text: typeText,
              style: item.isMaterial ? null : TextStyle(fontWeight: FontWeight.w600, color: p.warnFg),
            ),
            TextSpan(text: ' · $fileMeta'),
            if (item.dueDate != null)
              TextSpan(
                text: ' · prazo ${item.dueDate!.day.toString().padLeft(2, '0')}/${item.dueDate!.month.toString().padLeft(2, '0')}',
              ),
          ],
        ),
      ),
    );
  }
}
