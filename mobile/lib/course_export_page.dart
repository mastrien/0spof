import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';

import 'models.dart';
import 'store.dart';
import 'theme.dart';

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
        title: Text('Exportar · ${widget.course.shortname}', overflow: TextOverflow.ellipsis),
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

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                preview.fullname,
                style: Theme.of(context).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700),
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 8),
              TextField(
                decoration: InputDecoration(
                  hintText: 'Filtrar por título, PDF ou tipo…',
                  prefixIcon: const Icon(Icons.search),
                  isDense: true,
                  border: OutlineInputBorder(borderRadius: BorderRadius.circular(10)),
                ),
                onChanged: (v) => setState(() => _query = v),
              ),
              const SizedBox(height: 8),
              SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: Row(
                  children: [
                    ActionChip(
                      label: const Text('Materiais'),
                      onPressed: () =>
                          setState(() => _selected = applyExportPreset(preview, ExportPreset.materials)),
                    ),
                    const SizedBox(width: 6),
                    ActionChip(
                      label: const Text('Só PDFs'),
                      onPressed: () =>
                          setState(() => _selected = applyExportPreset(preview, ExportPreset.pdfs)),
                    ),
                    const SizedBox(width: 6),
                    FilterChip(
                      label: const Text('Incluir atividades'),
                      selected: activitiesOn,
                      onSelected: nonMaterials.isEmpty
                          ? null
                          : (on) => setState(() {
                                if (on) {
                                  _selected.addAll(nonMaterials.map((i) => i.id));
                                } else {
                                  _selected.removeAll(nonMaterials.map((i) => i.id));
                                }
                              }),
                    ),
                    const SizedBox(width: 6),
                    ActionChip(
                      label: const Text('Tudo'),
                      onPressed: () =>
                          setState(() => _selected = applyExportPreset(preview, ExportPreset.all)),
                    ),
                    const SizedBox(width: 6),
                    ActionChip(
                      label: const Text('Limpar'),
                      onPressed: () =>
                          setState(() => _selected = applyExportPreset(preview, ExportPreset.none)),
                    ),
                  ],
                ),
              ),
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                dense: true,
                title: const Text('Anexar arquivos baixados (PDFs e cópias offline)'),
                value: _includeFiles,
                onChanged: (v) => setState(() => _includeFiles = v),
              ),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: visibleSections.isEmpty
              ? const Center(
                  child: Text('Nenhum item corresponde ao filtro.', style: TextStyle(color: muted)),
                )
              : ListView.builder(
                  padding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
                  itemCount: visibleSections.length,
                  itemBuilder: (_, idx) {
                    final entry = visibleSections[idx];
                    final sec = entry.section;
                    final items = entry.items;
                    final selectedInSec = items.where((i) => _selected.contains(i.id)).length;
                    final bool? secValue = selectedInSec == 0
                        ? false
                        : selectedInSec == items.length
                            ? true
                            : null;

                    return Card(
                      margin: const EdgeInsets.only(bottom: 12),
                      child: Column(
                        children: [
                          CheckboxListTile(
                            tristate: true,
                            controlAffinity: ListTileControlAffinity.leading,
                            value: secValue,
                            title: Text(
                              sec.name,
                              style: const TextStyle(fontWeight: FontWeight.w700, fontSize: 14),
                            ),
                            subtitle: Text('${items.length} ${items.length == 1 ? 'item' : 'itens'}'),
                            onChanged: (_) => setState(() {
                              if (selectedInSec == items.length) {
                                _selected.removeAll(items.map((i) => i.id));
                              } else {
                                _selected.addAll(items.map((i) => i.id));
                              }
                            }),
                          ),
                          const Divider(height: 1),
                          for (final item in items) _buildItemTile(item),
                        ],
                      ),
                    );
                  },
                ),
        ),
        const Divider(height: 1),
        SafeArea(
          top: false,
          child: Padding(
            padding: const EdgeInsets.fromLTRB(16, 10, 16, 12),
            child: Row(
              children: [
                Expanded(
                  child: Text(
                    '${stats.selectedCount} de ${stats.totalCount} itens'
                    '${stats.pdfCount > 0 ? ' · ${stats.pdfCount} PDF${stats.pdfCount > 1 ? 's' : ''}' : ''}'
                    '${stats.totalBytes > 0 ? ' · ${formatFileSize(stats.totalBytes)}' : ''}',
                    style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
                  ),
                ),
                const SizedBox(width: 12),
                FilledButton.icon(
                  onPressed: _selected.isEmpty || _exporting ? null : () => _export(preview),
                  icon: _exporting
                      ? const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                        )
                      : const Icon(Icons.archive_outlined),
                  label: Text(_exporting ? 'Gerando…' : 'Exportar (.zip)'),
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildItemTile(ExportItemPreview item) {
    final checked = _selected.contains(item.id);
    final metaParts = <String>[
      _activityTypeLabel(item.activityType),
      if (item.hasLocalFile)
        'Offline (${formatFileSize(item.fileSizeBytes ?? 0)})'
      else if (item.url != null)
        'Apenas link'
      else
        'Somente registro',
    ];

    return CheckboxListTile(
      controlAffinity: ListTileControlAffinity.leading,
      value: checked,
      onChanged: (v) => setState(() {
        if (v == true) {
          _selected.add(item.id);
        } else {
          _selected.remove(item.id);
        }
      }),
      title: Row(
        children: [
          if (item.isPdf) ...[
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
              decoration: BoxDecoration(
                color: Colors.red.withValues(alpha: 0.12),
                borderRadius: BorderRadius.circular(5),
              ),
              child: const Text(
                'PDF',
                style: TextStyle(
                  color: Colors.redAccent,
                  fontWeight: FontWeight.w700,
                  fontSize: 10.5,
                ),
              ),
            ),
            const SizedBox(width: 6),
          ],
          Expanded(
            child: Text(
              item.title,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(color: checked ? null : muted),
            ),
          ),
        ],
      ),
      subtitle: Text(metaParts.join(' · ')),
    );
  }
}
