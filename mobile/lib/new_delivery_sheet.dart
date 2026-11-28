import 'package:flutter/material.dart';

import 'activity_actions.dart';
import 'models.dart';
import 'store.dart';
import 'theme.dart';

/// Folha inferior "Nova entrega": disciplina, tarefa (entre as com prazo), arquivo e carimbo.
Future<void> showNewDeliverySheet(BuildContext context, Store store, {FilePathPicker pick = pickFilePath}) async {
  final assigns = store.deadlines.where((d) => d.type == 'assign').toList();
  if (assigns.isEmpty) {
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('Nenhuma tarefa com prazo. Abra a tarefa pela disciplina.')),
    );
    return;
  }
  await showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    builder: (_) => _NewDeliverySheet(store: store, assigns: assigns, pick: pick),
  );
}

class _NewDeliverySheet extends StatefulWidget {
  final Store store;
  final List<Deadline> assigns;
  final FilePathPicker pick;
  const _NewDeliverySheet({required this.store, required this.assigns, required this.pick});
  @override
  State<_NewDeliverySheet> createState() => _NewDeliverySheetState();
}

class _NewDeliverySheetState extends State<_NewDeliverySheet> {
  static const _otherId = -1;

  bool _loading = true;
  final Map<int, String> _courseNames = {}; // id da disciplina -> nome (ordem de inserção)
  final Map<int, List<Deadline>> _byCourse = {};
  int? _courseId;
  int? _assignId;
  String? _path;
  bool _stamp = true;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _group();
  }

  /// Descobre a disciplina de cada tarefa pelo banco local; as que não forem achadas ficam em "Outras".
  Future<void> _group() async {
    final owner = <int, int>{};
    for (final c in widget.store.courses) {
      try {
        final sections = await widget.store.courseDetails(c.id);
        for (final s in sections) {
          for (final a in s.activities) {
            owner[a.id] = c.id;
          }
        }
      } catch (_) {
        // sem detalhes dessa disciplina: as tarefas caem em "Outras"
      }
    }
    final names = {for (final c in widget.store.courses) c.id: c.fullname};
    for (final d in widget.assigns) {
      final cid = owner[d.id] ?? _otherId;
      _byCourse.putIfAbsent(cid, () => []).add(d);
    }
    for (final c in widget.store.courses) {
      if (_byCourse.containsKey(c.id)) _courseNames[c.id] = names[c.id]!;
    }
    if (_byCourse.containsKey(_otherId)) _courseNames[_otherId] = 'Outras tarefas';
    if (!mounted) return;
    setState(() {
      _loading = false;
      _courseId = _courseNames.keys.first;
      _assignId = _byCourse[_courseId]!.first.id;
    });
  }

  Future<void> _pick() async {
    final path = await widget.pick();
    if (path == null || !mounted) return;
    setState(() => _path = path);
  }

  Future<void> _submit() async {
    final path = _path, assignId = _assignId;
    if (path == null || assignId == null || _busy) return;
    setState(() => _busy = true);
    await enqueueFile(context, widget.store, assignId, path, fetchTsa: _stamp);
    if (mounted) Navigator.pop(context);
  }

  Widget _label(String text, Pal p) => Padding(
        padding: const EdgeInsets.only(bottom: 6),
        child: Text(text, style: TextStyle(fontSize: 12, fontWeight: FontWeight.w600, color: p.mutFg)),
      );

  @override
  Widget build(BuildContext context) {
    final p = Pal.of(context);
    final scheme = Theme.of(context).colorScheme;
    final fileName = _path?.split(RegExp(r'[\\/]')).last;
    final tasks = _courseId == null ? const <Deadline>[] : _byCourse[_courseId]!;
    return Padding(
      padding: EdgeInsets.only(bottom: MediaQuery.of(context).viewInsets.bottom),
      child: SingleChildScrollView(
        padding: const EdgeInsets.fromLTRB(20, 10, 20, 24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Center(
              child: Container(
                width: 40,
                height: 4,
                decoration: BoxDecoration(color: p.fieldBorder, borderRadius: BorderRadius.circular(2)),
              ),
            ),
            const SizedBox(height: 10),
            Row(
              children: [
                Expanded(
                  child: Semantics(
                    header: true,
                    child: Text('Nova entrega',
                        style: TextStyle(fontSize: 20, fontWeight: FontWeight.w700, letterSpacing: -0.2, color: p.ink)),
                  ),
                ),
                IconButton(
                  tooltip: 'Fechar',
                  icon: const Icon(Icons.close),
                  style: IconButton.styleFrom(fixedSize: const Size(44, 44)),
                  onPressed: () => Navigator.pop(context),
                ),
              ],
            ),
            const SizedBox(height: 4),
            if (_loading)
              const Padding(padding: EdgeInsets.all(32), child: Center(child: CircularProgressIndicator()))
            else ...[
              _label('Disciplina', p),
              InputDecorator(
                decoration: const InputDecoration(contentPadding: EdgeInsets.symmetric(horizontal: 14, vertical: 0)),
                child: DropdownButtonHideUnderline(
                  child: DropdownButton<int>(
                    value: _courseId,
                    isExpanded: true,
                    borderRadius: BorderRadius.circular(12),
                    items: [
                      for (final e in _courseNames.entries)
                        DropdownMenuItem(value: e.key, child: Text(e.value, maxLines: 1, overflow: TextOverflow.ellipsis)),
                    ],
                    onChanged: (v) {
                      if (v == null) return;
                      setState(() {
                        _courseId = v;
                        _assignId = _byCourse[v]!.first.id;
                      });
                    },
                  ),
                ),
              ),
              const SizedBox(height: 14),
              _label('Tarefa', p),
              InputDecorator(
                decoration: const InputDecoration(contentPadding: EdgeInsets.symmetric(horizontal: 14, vertical: 0)),
                child: DropdownButtonHideUnderline(
                  child: DropdownButton<int>(
                    value: _assignId,
                    isExpanded: true,
                    borderRadius: BorderRadius.circular(12),
                    items: [
                      for (final d in tasks)
                        DropdownMenuItem(value: d.id, child: Text(d.title, maxLines: 1, overflow: TextOverflow.ellipsis)),
                    ],
                    onChanged: (v) => setState(() => _assignId = v),
                  ),
                ),
              ),
              const SizedBox(height: 14),
              _label('Arquivo', p),
              Semantics(
                button: true,
                label: fileName == null ? 'Escolher arquivo' : 'Arquivo escolhido: $fileName. Toque para trocar',
                excludeSemantics: true,
                onTap: _pick,
                child: InkWell(
                  onTap: _pick,
                  borderRadius: BorderRadius.circular(14),
                  child: Container(
                    constraints: const BoxConstraints(minHeight: 88),
                    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 18),
                    decoration: BoxDecoration(
                      color: p.softFill,
                      borderRadius: BorderRadius.circular(14),
                      border: Border.all(color: fileName == null ? p.fieldBorder : green, width: 1.5),
                    ),
                    child: Column(
                      children: [
                        Icon(fileName == null ? Icons.file_upload_outlined : Icons.insert_drive_file_outlined,
                            color: scheme.primary),
                        const SizedBox(height: 6),
                        Text(fileName ?? 'Escolher arquivo',
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                            textAlign: TextAlign.center,
                            style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: p.infoFg)),
                        const SizedBox(height: 2),
                        Text(fileName == null ? 'Qualquer formato aceito pela tarefa' : 'Toque para trocar',
                            style: TextStyle(fontSize: 12, color: p.muted)),
                      ],
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 14),
              Container(
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: p.fieldBorder),
                ),
                child: CheckboxListTile(
                  value: _stamp,
                  onChanged: _busy ? null : (v) => setState(() => _stamp = v ?? true),
                  controlAffinity: ListTileControlAffinity.leading,
                  contentPadding: const EdgeInsets.symmetric(horizontal: 8),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                  title: const Text('Gerar carimbo de tempo', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w700)),
                  subtitle: Text('Tenta obter um token de tempo RFC 3161 (assinatura não verificada); se a autoridade estiver fora do ar, só o hash é registrado.',
                      style: TextStyle(fontSize: 12, height: 1.4, color: p.muted)),
                ),
              ),
              const SizedBox(height: 14),
              FilledButton(
                style: FilledButton.styleFrom(minimumSize: const Size.fromHeight(52)),
                onPressed: (_path == null || _assignId == null || _busy) ? null : _submit,
                child: Text(_busy
                    ? 'Registrando…'
                    : _stamp
                        ? 'Registrar entrega'
                        : 'Registrar sem carimbo'),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
