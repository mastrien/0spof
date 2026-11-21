import 'package:flutter/material.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import 'store.dart';

/// Lê o QR code do app desktop (ou aceita o código colado) e pareia.
class PairPage extends StatefulWidget {
  final Store store;
  const PairPage({super.key, required this.store});
  @override
  State<PairPage> createState() => _PairPageState();
}

class _PairPageState extends State<PairPage> {
  final _manual = TextEditingController();
  bool _busy = false;
  String? _error;

  Future<void> _pair(String code) async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await widget.store.pairWithPc(code);
      if (mounted) Navigator.pop(context);
    } catch (e) {
      if (mounted) setState(() => _error = '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  void dispose() {
    _manual.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(title: const Text('Parear com o PC')),
        body: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            const Text('No app desktop, abra Ajustes e aponte a câmera para o QR code. O celular precisa estar na mesma rede do PC.'),
            const SizedBox(height: 16),
            ClipRRect(
              borderRadius: BorderRadius.circular(14),
              child: SizedBox(
                height: 300,
                child: MobileScanner(
                  onDetect: (capture) {
                    final raw = capture.barcodes.map((b) => b.rawValue).whereType<String>().firstOrNull;
                    if (raw != null) _pair(raw);
                  },
                  errorBuilder: (context, error) => Center(
                    child: Padding(
                      padding: const EdgeInsets.all(24),
                      child: Text('Câmera indisponível. Cole o código abaixo.\n(${error.errorCode.name})', textAlign: TextAlign.center),
                    ),
                  ),
                ),
              ),
            ),
            const SizedBox(height: 16),
            TextField(
              controller: _manual,
              minLines: 2,
              maxLines: 4,
              decoration: const InputDecoration(border: OutlineInputBorder(), labelText: 'Ou cole o código de pareamento'),
            ),
            const SizedBox(height: 12),
            FilledButton(
              onPressed: _busy ? null : () => _pair(_manual.text),
              child: _busy ? const SizedBox(height: 18, width: 18, child: CircularProgressIndicator(strokeWidth: 2)) : const Text('Parear'),
            ),
            if (_error != null) Padding(padding: const EdgeInsets.only(top: 12), child: Text(_error!, style: const TextStyle(color: Colors.redAccent))),
          ],
        ),
      );
}
