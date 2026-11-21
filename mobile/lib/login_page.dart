import 'package:flutter/material.dart';
import 'package:webview_flutter/webview_flutter.dart';

import 'native_cookies.dart';
import 'src/rust/api/spof.dart' as rust;
import 'store.dart';

/// Login institucional (Moodle / SUAP) num WebView. Ao chegar numa página autenticada,
/// lê o cookie `MoodleSession` (HttpOnly) pelo CookieManager nativo e fecha.
class LoginPage extends StatefulWidget {
  final Store store;
  const LoginPage({super.key, required this.store});
  @override
  State<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> {
  final _url = TextEditingController();
  WebViewController? _web;
  String? _base;
  bool _capturing = false;
  int _progress = 0;
  String? _error;

  @override
  void initState() {
    super.initState();
    widget.store.savedMoodleUrl().then((u) {
      if (mounted) setState(() => _url.text = u);
    });
  }

  @override
  void dispose() {
    _url.dispose();
    super.dispose();
  }

  String? _normalize(String raw) {
    var u = raw.trim();
    if (u.isEmpty) return null;
    if (!u.startsWith('http://') && !u.startsWith('https://')) u = 'https://$u';
    final uri = Uri.tryParse(u);
    if (uri == null || uri.host.isEmpty) return null;
    return u.replaceFirst(RegExp(r'/+$'), '');
  }

  void _start() {
    final base = _normalize(_url.text);
    if (base == null) {
      setState(() => _error = 'URL inválida');
      return;
    }
    final web = WebViewController()
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setNavigationDelegate(NavigationDelegate(
        onProgress: (p) => mounted ? setState(() => _progress = p) : null,
        onPageFinished: _onPage,
        onUrlChange: (c) => c.url == null ? null : _onPage(c.url!),
        onWebResourceError: (e) {
          if (e.isForMainFrame ?? false) setState(() => _error = 'Não foi possível abrir a página: ${e.description}');
        },
      ))
      ..loadRequest(Uri.parse('$base/login/index.php'));
    setState(() {
      _error = null;
      _base = base;
      _web = web;
    });
  }

  Future<void> _onPage(String url) async {
    final base = _base;
    if (base == null || _capturing) return;
    if (!rust.isAuthenticatedUrl(url: url)) return;
    _capturing = true;
    try {
      final value = await NativeCookies.get(base, 'MoodleSession') ?? '';
      if (value.isEmpty) {
        _capturing = false;
        return;
      }
      await widget.store.loginMoodle(base, value);
      if (mounted) Navigator.pop(context, true);
    } catch (e) {
      _capturing = false;
      if (mounted) setState(() => _error = '$e');
    }
  }

  @override
  Widget build(BuildContext context) {
    final web = _web;
    return Scaffold(
      appBar: AppBar(
        title: const Text('Entrar no Moodle'),
        bottom: web != null && _progress < 100
            ? PreferredSize(preferredSize: const Size.fromHeight(2), child: LinearProgressIndicator(value: _progress / 100, minHeight: 2))
            : null,
      ),
      body: web == null
          ? ListView(
              padding: const EdgeInsets.all(16),
              children: [
                const Text('Entre com sua conta institucional (SUAP / Moodle). O 0spof guarda só o cookie de sessão, '
                    'cifrado no celular, e nunca vê sua senha.'),
                const SizedBox(height: 16),
                TextField(
                  controller: _url,
                  keyboardType: TextInputType.url,
                  decoration: const InputDecoration(border: OutlineInputBorder(), labelText: 'URL do Moodle'),
                  onSubmitted: (_) => _start(),
                ),
                const SizedBox(height: 12),
                FilledButton(onPressed: _start, child: const Text('Continuar')),
                if (_error != null) Padding(padding: const EdgeInsets.only(top: 12), child: Text(_error!, style: const TextStyle(color: Colors.redAccent))),
              ],
            )
          : Column(
              children: [
                if (_error != null)
                  MaterialBanner(content: Text(_error!), actions: [TextButton(onPressed: () => setState(() => _error = null), child: const Text('OK'))]),
                Expanded(child: WebViewWidget(controller: web)),
              ],
            ),
    );
  }
}
