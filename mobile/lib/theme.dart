import 'package:flutter/material.dart';

/// Paleta do desktop (`style.css`): azul 0spof sobre fundo claro, navy no escuro.
const blue = Color(0xFF3768ED);
const navy = Color(0xFF17233F);
const canvas = Color(0xFFF6F7FB);
const ink = Color(0xFF18223A);
const muted = Color(0xFF6D7890);

ThemeData buildTheme(Brightness b) {
  final dark = b == Brightness.dark;
  final scheme = ColorScheme.fromSeed(seedColor: blue, brightness: b);
  return ThemeData(
    useMaterial3: true,
    colorScheme: scheme,
    scaffoldBackgroundColor: dark ? const Color(0xFF0F1626) : canvas,
    appBarTheme: AppBarTheme(
      backgroundColor: dark ? navy : Colors.white,
      foregroundColor: dark ? Colors.white : ink,
      elevation: 0,
      scrolledUnderElevation: 0.5,
    ),
    cardTheme: CardThemeData(
      elevation: 0,
      color: dark ? navy : Colors.white,
      margin: EdgeInsets.zero,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(14),
        side: BorderSide(color: dark ? Colors.white12 : const Color(0xFFE8EBF2)),
      ),
    ),
    navigationBarTheme: NavigationBarThemeData(
      backgroundColor: dark ? navy : Colors.white,
      indicatorColor: blue.withValues(alpha: 0.18),
    ),
  );
}
