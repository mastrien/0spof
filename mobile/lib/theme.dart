import 'package:flutter/material.dart';

/// Paleta do redesign (alinhada ao desktop): ação azul, tinta navy, fundo frio.
/// Verde só para "protegido/confirmado"; âmbar para atenção.
const blue = Color(0xFF2F5BEA);
const navy = Color(0xFF14203A);
const canvas = Color(0xFFF5F6FA);
const ink = Color(0xFF14203A);
const muted = Color(0xFF5B6783);
const green = Color(0xFF1F8A5B);
const amber = Color(0xFFB45309);

/// Tons semânticos dos chips e avisos.
enum Tone { ok, warn, info, mut, err }

/// Cores resolvidas conforme o brilho do tema. Use `Pal.of(context)`.
class Pal {
  final Color bg, surface, border, divider, ink, muted, fieldBorder, softFill, hero, onHero, heroMuted;
  final Color okBg, okFg, warnBg, warnFg, infoBg, infoFg, mutBg, mutFg, errBg, errFg;
  final Color warnCardBg, warnCardBorder, okCardBg, okCardBorder, errBorder;

  const Pal({
    required this.bg,
    required this.surface,
    required this.border,
    required this.divider,
    required this.ink,
    required this.muted,
    required this.fieldBorder,
    required this.softFill,
    required this.hero,
    required this.onHero,
    required this.heroMuted,
    required this.okBg,
    required this.okFg,
    required this.warnBg,
    required this.warnFg,
    required this.infoBg,
    required this.infoFg,
    required this.mutBg,
    required this.mutFg,
    required this.errBg,
    required this.errFg,
    required this.warnCardBg,
    required this.warnCardBorder,
    required this.okCardBg,
    required this.okCardBorder,
    required this.errBorder,
  });

  static const light = Pal(
    bg: Color(0xFFF5F6FA),
    surface: Colors.white,
    border: Color(0xFFE3E7F0),
    divider: Color(0xFFEEF1F7),
    ink: Color(0xFF14203A),
    muted: Color(0xFF5B6783),
    fieldBorder: Color(0xFFD5DBE8),
    softFill: Color(0xFFF8F9FC),
    hero: Color(0xFF14203A),
    onHero: Colors.white,
    heroMuted: Color(0xFFC5D0EA),
    okBg: Color(0xFFE3F4EC),
    okFg: Color(0xFF146240),
    warnBg: Color(0xFFFCF0D8),
    warnFg: Color(0xFF7A4B0A),
    infoBg: Color(0xFFE6ECFD),
    infoFg: Color(0xFF2342B8),
    mutBg: Color(0xFFEEF1F7),
    mutFg: Color(0xFF4A5672),
    errBg: Color(0xFFFBE5E1),
    errFg: Color(0xFF9B2C1A),
    warnCardBg: Color(0xFFFFFBF1),
    warnCardBorder: Color(0xFFF0D9A8),
    okCardBg: Color(0xFFF1FAF5),
    okCardBorder: Color(0xFFBFE3D0),
    errBorder: Color(0xFFEBC4BC),
  );

  static const dark = Pal(
    bg: Color(0xFF0E1424),
    surface: Color(0xFF172038),
    border: Color(0xFF2A3452),
    divider: Color(0xFF222B45),
    ink: Color(0xFFE8ECF6),
    muted: Color(0xFFA3AEC8),
    fieldBorder: Color(0xFF3A4668),
    softFill: Color(0xFF131B30),
    hero: Color(0xFF22305A),
    onHero: Colors.white,
    heroMuted: Color(0xFFC5D0EA),
    okBg: Color(0xFF14382A),
    okFg: Color(0xFF8FE0B8),
    warnBg: Color(0xFF3E2F10),
    warnFg: Color(0xFFF3CE85),
    infoBg: Color(0xFF1E2C5E),
    infoFg: Color(0xFFB3C5FF),
    mutBg: Color(0xFF222B45),
    mutFg: Color(0xFFB4BED6),
    errBg: Color(0xFF40201C),
    errFg: Color(0xFFF4B1A5),
    warnCardBg: Color(0xFF2B2210),
    warnCardBorder: Color(0xFF5E4A1E),
    okCardBg: Color(0xFF12301F),
    okCardBorder: Color(0xFF245A40),
    errBorder: Color(0xFF6B3A33),
  );

  static Pal of(BuildContext context) =>
      Theme.of(context).brightness == Brightness.dark ? dark : light;

  Color bgOf(Tone t) => switch (t) {
        Tone.ok => okBg,
        Tone.warn => warnBg,
        Tone.info => infoBg,
        Tone.mut => mutBg,
        Tone.err => errBg,
      };

  Color fgOf(Tone t) => switch (t) {
        Tone.ok => okFg,
        Tone.warn => warnFg,
        Tone.info => infoFg,
        Tone.mut => mutFg,
        Tone.err => errFg,
      };
}

ThemeData buildTheme(Brightness b) {
  final dark = b == Brightness.dark;
  final p = dark ? Pal.dark : Pal.light;
  final action = dark ? const Color(0xFF8CA8FF) : blue;
  final onAction = dark ? const Color(0xFF0E1424) : Colors.white;
  final scheme = ColorScheme.fromSeed(seedColor: blue, brightness: b).copyWith(
    primary: action,
    onPrimary: onAction,
    primaryContainer: p.infoBg,
    onPrimaryContainer: p.infoFg,
    surface: p.surface,
    onSurface: p.ink,
    onSurfaceVariant: p.muted,
    outline: p.fieldBorder,
    outlineVariant: p.border,
    error: dark ? const Color(0xFFF4B1A5) : const Color(0xFF9B2C1A),
    surfaceContainerLowest: p.surface,
    surfaceContainerLow: p.surface,
    surfaceContainer: p.surface,
    surfaceContainerHigh: p.surface,
    surfaceContainerHighest: p.mutBg,
  );
  return ThemeData(
    useMaterial3: true,
    brightness: b,
    colorScheme: scheme,
    scaffoldBackgroundColor: p.bg,
    dividerColor: p.divider,
    appBarTheme: AppBarTheme(
      backgroundColor: p.surface,
      foregroundColor: p.ink,
      surfaceTintColor: Colors.transparent,
      elevation: 0,
      scrolledUnderElevation: 0,
      shape: Border(bottom: BorderSide(color: p.border)),
      titleTextStyle: TextStyle(color: p.ink, fontSize: 17, fontWeight: FontWeight.w700),
    ),
    cardTheme: CardThemeData(
      elevation: 0,
      color: p.surface,
      surfaceTintColor: Colors.transparent,
      margin: EdgeInsets.zero,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(16),
        side: BorderSide(color: p.border),
      ),
    ),
    bottomSheetTheme: BottomSheetThemeData(
      backgroundColor: p.surface,
      surfaceTintColor: Colors.transparent,
      showDragHandle: false,
      shape: const RoundedRectangleBorder(borderRadius: BorderRadius.vertical(top: Radius.circular(24))),
    ),
    navigationBarTheme: NavigationBarThemeData(
      height: 72,
      backgroundColor: p.surface,
      surfaceTintColor: Colors.transparent,
      indicatorColor: p.infoBg,
      indicatorShape: const StadiumBorder(),
      labelTextStyle: WidgetStatePropertyAll(
        TextStyle(fontSize: 12, fontWeight: FontWeight.w600, color: p.ink),
      ),
      iconTheme: WidgetStateProperty.resolveWith(
        (s) => IconThemeData(color: s.contains(WidgetState.selected) ? p.infoFg : p.muted),
      ),
    ),
    floatingActionButtonTheme: FloatingActionButtonThemeData(
      backgroundColor: action,
      foregroundColor: onAction,
      extendedTextStyle: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        minimumSize: const Size(44, 48),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14)),
        textStyle: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        minimumSize: const Size(44, 44),
        foregroundColor: p.ink,
        side: BorderSide(color: p.fieldBorder),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        textStyle: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(minimumSize: const Size(44, 44)),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: p.surface,
      contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      hintStyle: TextStyle(color: p.muted, fontSize: 14),
      border: OutlineInputBorder(borderRadius: BorderRadius.circular(12), borderSide: BorderSide(color: p.fieldBorder)),
      enabledBorder: OutlineInputBorder(borderRadius: BorderRadius.circular(12), borderSide: BorderSide(color: p.fieldBorder)),
      focusedBorder: OutlineInputBorder(borderRadius: BorderRadius.circular(12), borderSide: BorderSide(color: action, width: 2)),
    ),
    checkboxTheme: CheckboxThemeData(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
      side: BorderSide(color: p.muted, width: 1.5),
    ),
    chipTheme: ChipThemeData(
      backgroundColor: p.surface,
      selectedColor: p.infoBg,
      side: BorderSide(color: p.fieldBorder),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(18)),
      labelStyle: TextStyle(color: p.ink, fontSize: 13, fontWeight: FontWeight.w600),
    ),
    snackBarTheme: SnackBarThemeData(
      behavior: SnackBarBehavior.floating,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
    ),
    dialogTheme: DialogThemeData(
      backgroundColor: p.surface,
      surfaceTintColor: Colors.transparent,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
    ),
  );
}
