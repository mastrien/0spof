import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:open_filex/open_filex.dart';
import 'package:url_launcher/url_launcher.dart';

import 'store.dart';

void _toast(BuildContext context, String msg) =>
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg)));

/// Abre um arquivo baixado no app padrão do Android (via FileProvider).
Future<void> openLocalFile(BuildContext context, String path) async {
  final r = await OpenFilex.open(path);
  if (r.type != ResultType.done && context.mounted) _toast(context, 'Não foi possível abrir: ${r.message}');
}

/// Ações de uma atividade: abrir no Moodle, abrir/baixar material, enfileirar entrega.
Future<void> showActivityActions(
  BuildContext context,
  Store store, {
  required int id,
  required String title,
  required String type,
  String? url,
  String? localPath,
}) {
  final canDownload = store.session.authenticated && (type == 'resource' || type == 'folder' || type == 'url');
  return showModalBottomSheet(
    context: context,
    showDragHandle: true,
    builder: (sheet) => SafeArea(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 0, 20, 8),
            child: Text(title, style: Theme.of(sheet).textTheme.titleMedium, maxLines: 3, overflow: TextOverflow.ellipsis),
          ),
          if (localPath != null)
            ListTile(
              leading: const Icon(Icons.file_open_outlined),
              title: const Text('Abrir material baixado'),
              onTap: () {
                Navigator.pop(sheet);
                openLocalFile(context, localPath);
              },
            ),
          if (canDownload)
            ListTile(
              leading: const Icon(Icons.download_outlined),
              title: Text(localPath == null ? 'Baixar para o celular' : 'Baixar de novo'),
              onTap: () {
                Navigator.pop(sheet);
                _download(context, store, id);
              },
            ),
          if (type == 'assign')
            ListTile(
              leading: const Icon(Icons.outbox_outlined),
              title: const Text('Enfileirar entrega'),
              subtitle: const Text('Hash SHA-256 + carimbo de tempo RFC 3161'),
              onTap: () {
                Navigator.pop(sheet);
                pickAndEnqueue(context, store, id);
              },
            ),
          if (url != null)
            ListTile(
              leading: const Icon(Icons.open_in_browser),
              title: const Text('Abrir no Moodle'),
              onTap: () {
                Navigator.pop(sheet);
                launchUrl(Uri.parse(url), mode: LaunchMode.externalApplication);
              },
            ),
          const SizedBox(height: 8),
        ],
      ),
    ),
  );
}

Future<void> _download(BuildContext context, Store store, int id) async {
  _toast(context, 'Baixando…');
  try {
    final path = await store.downloadMedia(id);
    if (context.mounted) {
      ScaffoldMessenger.of(context).hideCurrentSnackBar();
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
        content: const Text('Material salvo no celular.'),
        action: SnackBarAction(label: 'Abrir', onPressed: () => openLocalFile(context, path)),
      ));
    }
  } catch (e) {
    if (context.mounted) _toast(context, 'Falha no download: $e');
  }
}

/// Seletor de arquivo (injetável em testes): devolve o caminho escolhido ou nulo.
typedef FilePathPicker = Future<String?> Function();

Future<String?> pickFilePath() async {
  final picked = await FilePicker.pickFiles();
  return picked.isEmpty ? null : picked.first.path;
}

/// Escolhe um arquivo e o enfileira para a tarefa `assignId` (com carimbo RFC 3161 por padrão).
Future<void> pickAndEnqueue(BuildContext context, Store store, int assignId, {FilePathPicker pick = pickFilePath}) async {
  final path = await pick();
  if (path == null || !context.mounted) return;
  await enqueueFile(context, store, assignId, path);
}

/// Enfileira o arquivo já escolhido; com `fetchTsa` pede o carimbo RFC 3161.
Future<void> enqueueFile(BuildContext context, Store store, int assignId, String path, {bool fetchTsa = true}) async {
  _toast(context, fetchTsa ? 'Calculando hash e pedindo carimbo de tempo…' : 'Calculando hash…');
  try {
    final r = await store.enqueueSubmission(assignId, path, fetchTsa: fetchTsa);
    if (!context.mounted) return;
    ScaffoldMessenger.of(context).hideCurrentSnackBar();
    _toast(
      context,
      r.hasTsaToken
          ? 'Entrega #${r.outboxId} na fila, com carimbo RFC 3161.'
          : fetchTsa
              ? 'Entrega #${r.outboxId} na fila. Sem carimbo (TSA fora do ar); o hash local vale como prova.'
              : 'Entrega #${r.outboxId} na fila, sem carimbo de tempo.',
    );
  } catch (e) {
    if (context.mounted) _toast(context, 'Falha ao enfileirar: $e');
  }
}

Future<void> showProof(BuildContext context, Store store, int outboxId) async {
  try {
    final p = await store.proof(outboxId);
    if (!context.mounted) return;
    await showDialog(
      context: context,
      builder: (d) => AlertDialog(
        title: Text('Comprovante · ${p.filename}', maxLines: 2, overflow: TextOverflow.ellipsis),
        content: SingleChildScrollView(
          child: SelectableText(p.report, style: const TextStyle(fontFamily: 'monospace', fontSize: 12)),
        ),
        actions: [
          TextButton(
            onPressed: () {
              Clipboard.setData(ClipboardData(text: p.report));
              Navigator.pop(d);
              _toast(context, 'Comprovante copiado.');
            },
            child: const Text('Copiar'),
          ),
          TextButton(onPressed: () => Navigator.pop(d), child: const Text('Fechar')),
        ],
      ),
    );
  } catch (e) {
    if (context.mounted) _toast(context, 'Erro ao gerar comprovante: $e');
  }
}
