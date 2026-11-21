package com.mastrien.spof_mobile

import android.webkit.CookieManager
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

/**
 * Canal `zerospof/cookies`: lê e apaga cookies do WebView do sistema.
 * O `MoodleSession` é HttpOnly, então o JavaScript da página não o enxerga; o CookieManager nativo sim.
 */
class MainActivity : FlutterActivity() {
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "zerospof/cookies").setMethodCallHandler { call, result ->
            val manager = CookieManager.getInstance()
            when (call.method) {
                "get" -> {
                    val url = call.argument<String>("url")
                    val name = call.argument<String>("name")
                    if (url == null || name == null) {
                        result.error("args", "url e name são obrigatórios", null)
                    } else {
                        val value = manager.getCookie(url)
                            ?.split(";")
                            ?.map { it.trim() }
                            ?.firstOrNull { it.startsWith("$name=") }
                            ?.substringAfter("=")
                        result.success(value)
                    }
                }
                "clear" -> manager.removeAllCookies { result.success(it) }
                else -> result.notImplemented()
            }
        }
    }
}
