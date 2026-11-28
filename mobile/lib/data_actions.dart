import 'dart:convert';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';

import 'course_export_page.dart';
import 'models.dart';
import 'store.dart';
import 'theme.dart';

void _toast(BuildContext context, String msg) =>
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg)));

/// Exportar / importar / apagar: o mesmo backup JSON do desktop, então dá para levar dados entre os dois.
Future<void> exportBackup(BuildContext context, Store store) async {
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

/// Escolhe uma disciplina e abre a seleção de itens da exportação.
Future<void> exportCoursePicker(BuildContext context, Store store) async {
  final courses = store.courses;
  if (courses.isEmpty) {
    _toast(context, 'Nenhuma disciplina salva para exportar.');
    return;
  }
  final chosen = await showModalBottomSheet<Course>(
    context: context,
    isScrollControlled: true,
    builder: (sheet) {
      final p = Pal.of(sheet);
      return SafeArea(
        child: ConstrainedBox(
          constraints: BoxConstraints(maxHeight: MediaQuery.of(sheet).size.height * 0.8),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Center(
                child: Container(
                  margin: const EdgeInsets.only(top: 10),
                  width: 40,
                  height: 4,
                  decoration: BoxDecoration(color: p.fieldBorder, borderRadius: BorderRadius.circular(2)),
                ),
              ),
              const Padding(
                padding: EdgeInsets.fromLTRB(20, 14, 20, 8),
                child: Text('Escolha a disciplina para compartilhar',
                    style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
              ),
              Flexible(
                child: ListView(
                  shrinkWrap: true,
                  children: [
                    for (final c in courses)
                      ListTile(
                        minVerticalPadding: 8,
                        leading: Icon(Icons.school_outlined, color: p.muted),
                        title: Text(c.fullname, maxLines: 2, overflow: TextOverflow.ellipsis),
                        subtitle: Text('${c.sections} seções · ${c.activities} atividades'),
                        onTap: () => Navigator.pop(sheet, c),
                      ),
                  ],
                ),
              ),
            ],
          ),
        ),
      );
    },
  );
  if (chosen != null && context.mounted) {
    await Navigator.push(
      context,
      MaterialPageRoute(builder: (_) => CourseExportPage(store: store, course: chosen)),
    );
  }
}

/// Importa um backup `.json` ou pacote de disciplina `.zip`.
Future<void> importPackage(BuildContext context, Store store) async {
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

Future<void> wipeLocalData(BuildContext context, Store store) async {
  final ok = await showDialog<bool>(
    context: context,
    builder: (d) => AlertDialog(
      title: const Text('Apagar todos os dados locais?'),
      content: const Text('Disciplinas, materiais e a fila de entregas deste celular serão apagados. '
          'A sessão do Moodle e o pareamento com o PC continuam.'),
      actions: [
        TextButton(onPressed: () => Navigator.pop(d, false), child: const Text('Cancelar')),
        FilledButton(
          style: FilledButton.styleFrom(
            backgroundColor: Pal.of(d).errFg,
            foregroundColor: Pal.of(d).surface,
          ),
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
