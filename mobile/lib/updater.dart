import 'dart:convert';
import 'dart:io';

import 'package:http/http.dart' as http;
import 'package:ota_update/ota_update.dart';
import 'package:package_info_plus/package_info_plus.dart';

/// Atualização automática no Android (sem Play Store): consulta o último release do GitHub,
/// baixa o APK e abre o instalador do sistema, que pede a confirmação do usuário.
class AppUpdate {
  final String version;
  final String apkUrl;
  final String? sha256;

  AppUpdate(this.version, this.apkUrl, this.sha256);

  static const _latest = 'https://api.github.com/repos/mastrien/0spof/releases/latest';

  /// Consulta o último release. Distingue "já está na última versão" de "não deu para verificar"
  /// (sem rede, erro HTTP, JSON inesperado, release sem APK) para a tela não afirmar o que não sabe.
  /// `client`, `currentVersion` e `android` existem para os testes.
  static Future<UpdateCheck> check({http.Client? client, Future<String> Function()? currentVersion, bool? android}) async {
    if (!(android ?? Platform.isAndroid)) return const UpdateCheck(UpdateStatus.unsupported);
    final http_ = client ?? http.Client();
    try {
      final response = await http_
          .get(Uri.parse(_latest), headers: {'Accept': 'application/vnd.github+json'})
          .timeout(const Duration(seconds: 10));
      if (response.statusCode != 200) return const UpdateCheck(UpdateStatus.failed);
      final release = jsonDecode(response.body) as Map<String, dynamic>;
      final latest = (release['tag_name'] as String).replaceFirst('v', '');
      final current = await (currentVersion ?? () async => (await PackageInfo.fromPlatform()).version)();
      if (!isNewer(latest, current)) return const UpdateCheck(UpdateStatus.upToDate);

      final apk = (release['assets'] as List).cast<Map<String, dynamic>>().where((a) => (a['name'] as String).endsWith('.apk')).firstOrNull;
      if (apk == null) return const UpdateCheck(UpdateStatus.failed);
      // O GitHub informa o sha256 de cada arquivo ("sha256:<hex>"); assim o APK baixado é conferido.
      final digest = apk['digest'] as String?;
      final update = AppUpdate(latest, apk['browser_download_url'] as String, digest?.startsWith('sha256:') == true ? digest!.substring(7) : null);
      return UpdateCheck(UpdateStatus.available, update);
    } catch (_) {
      return const UpdateCheck(UpdateStatus.failed);
    } finally {
      if (client == null) http_.close();
    }
  }

  /// Baixa e abre o instalador. Os eventos trazem o progresso do download (0–100).
  Stream<OtaEvent> install() => OtaUpdate().execute(apkUrl, destinationFilename: '0spof-$version.apk', sha256checksum: sha256);
}

enum UpdateStatus { available, upToDate, failed, unsupported }

/// Resultado de [AppUpdate.check]; `update` só vem preenchido com `available`.
class UpdateCheck {
  final UpdateStatus status;
  final AppUpdate? update;
  const UpdateCheck(this.status, [this.update]);
}

/// `1.10.0` > `1.9.3`. Partes ausentes valem 0; sufixos (`-beta`) são ignorados.
bool isNewer(String candidate, String current) {
  List<int> parts(String v) => v.split('-').first.split('.').map((p) => int.tryParse(p) ?? 0).toList();
  final a = parts(candidate), b = parts(current);
  for (var i = 0; i < 3; i++) {
    final x = i < a.length ? a[i] : 0, y = i < b.length ? b[i] : 0;
    if (x != y) return x > y;
  }
  return false;
}
