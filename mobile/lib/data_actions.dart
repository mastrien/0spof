import 'dart:convert';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';

import 'course_export_page.dart';
import 'models.dart';
import 'store.dart';

void _toast(BuildContext context, String msg) =>
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg)));

/// Exportar / importar / apagar: o mesmo backup JSON do desktop, então dá para levar dados entre os dois.
class DataCard extends StatelessWidget {
  final Store store;
  const DataCard({super.key, required this.store});

  Future<void> _export(BuildContext context) async {
    try {
      final json = await store.exportData();
      final d = DateTime.now();
      final name = '0spof-backup-${d.year}${d.month.toString().padLeft(2, '0')}${d.day.toString().padLeft(2, '0')}.json';
      final uri = await FilePicker.saveFile(
        fileName: name,
        bytes: utf8.encode(json),
        mimeType: 'application/json',
        dialogTitle: 'Salvar backup do 0spof',
      );
      if (uri != null && context.mounted) _toast(context, 'Backup salvo.');
    } catch (e) {
      if (context.mounted) _toast(context, 'Falha ao exportar: $e');
    }
  }

  Future<void> _exportCourse(BuildContext context) async {
    final courses = store.courses;
    if (courses.isEmpty) {
      _toast(context, 'Nenhuma disciplina salva para exportar.');
      return;
    }
    final chosen = await showModalBottomSheet<Course>(
      context: context,
      showDragHandle: true,
      builder: (sheet) => SafeArea(
        child: ListView(
          shrinkWrap: true,
          children: [
            const ListTile(
              title: Text('Escolha a disciplina para compartilhar',
                  style: TextStyle(fontWeight: FontWeight.w700)),
            ),
            for (final c in courses)
              ListTile(
                leading: const Icon(Icons.school_outlined),
                title: Text(c.fullname, maxLines: 2, overflow: TextOverflow.ellipsis),
                subtitle: Text('${c.sections} seções · ${c.activities} atividades'),
                onTap: () => Navigator.pop(sheet, c),
              ),
          ],
        ),
      ),
    );
    if (chosen != null && context.mounted) {
      await Navigator.push(
        context,
        MaterialPageRoute(builder: (_) => CourseExportPage(store: store, course: chosen)),
      );
    }
  }

  Future<void> _import(BuildContext context) async {
    try {
      final picked = await FilePicker.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['json', 'zip'],
      );
      final path = picked.isEmpty ? null : picked.first.path;
      if (path == null) return;
      final summary = await store.importPackageFile(path);
      if (context.mounted) _toast(context, 'Importado: $summary');
    } catch (e) {
      if (context.mounted) _toast(context, 'Falha ao importar: $e');
    }
  }

  Future<void> _wipe(BuildContext context) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (d) => AlertDialog(
        title: const Text('Apagar todos os dados locais?'),
        content: const Text('Disciplinas, materiais e a fila de entregas deste celular serão apagados. '
            'A sessão do Moodle e o pareamento com o PC continuam.'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(d, false), child: const Text('Cancelar')),
          FilledButton(
            style: FilledButton.styleFrom(backgroundColor: Colors.redAccent),
            onPressed: () => Navigator.pop(d, true),
            child: const Text('Apagar'),
          ),
        ],
      ),
    );
    if (ok != true) return;
    try {
      await store.wipeAllData();
      if (context.mounted) _toast(context, 'Dados locais apagados.');
    } catch (e) {
      if (context.mounted) _toast(context, 'Falha ao apagar: $e');
    }
  }

  @override
  Widget build(BuildContext context) => Card(
        child: Column(
          children: [
            ListTile(
              leading: const Icon(Icons.share_outlined),
              title: const Text('Exportar disciplina (.zip)'),
              subtitle: const Text('Escolha materiais e PDFs para compartilhar'),
              onTap: () => _exportCourse(context),
            ),
            ListTile(leading: const Icon(Icons.upload_file), title: const Text('Exportar backup (JSON)'), onTap: () => _export(context)),
            ListTile(
              leading: const Icon(Icons.download_for_offline_outlined),
              title: const Text('Importar backup ou disciplina (.json / .zip)'),
              onTap: () => _import(context),
            ),
            ListTile(
              leading: const Icon(Icons.delete_forever_outlined, color: Colors.redAccent),
              title: const Text('Apagar dados locais', style: TextStyle(color: Colors.redAccent)),
              onTap: () => _wipe(context),
            ),
          ],
        ),
      );
}
