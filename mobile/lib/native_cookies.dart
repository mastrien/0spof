import 'package:flutter/services.dart';

/// Ponte para o CookieManager nativo do Android (ver `MainActivity.kt`).
class NativeCookies {
  static const _channel = MethodChannel('zerospof/cookies');

  static Future<String?> get(String url, String name) =>
      _channel.invokeMethod<String>('get', {'url': url, 'name': name});

  static Future<void> clear() => _channel.invokeMethod('clear');
}
