import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:spof_mobile_app/format.dart';
import 'package:spof_mobile_app/theme.dart';

void main() {
  test('fmtToday em português', () {
    expect(fmtToday(DateTime(2026, 11, 28)), 'Sábado, 28 de novembro');
  });

  test('datas curtas', () {
    final d = DateTime(2026, 11, 30, 23, 59);
    expect(weekdayAbbr(d), 'SEG');
    expect(fmtShortDate(d), 'seg, 30/11');
    expect(fmtDueLong(d), 'seg, 30/11 às 23:59');
  });

  test('fmtCountdown', () {
    final now = DateTime(2026, 11, 28, 15, 59);
    expect(fmtCountdown(DateTime(2026, 11, 30, 23, 59), now: now), '2d 08h');
    expect(fmtCountdown(DateTime(2026, 11, 28, 21, 11), now: now), '5h 12min');
    expect(fmtCountdown(DateTime(2026, 11, 28, 10, 0), now: now), 'venceu');
  });

  test('fmtAgo', () {
    final now = DateTime(2026, 11, 28, 12, 0);
    expect(fmtAgo(DateTime(2026, 11, 28, 11, 48), now: now), 'há 12 min');
    expect(fmtAgo(DateTime(2026, 11, 28, 9, 0), now: now), 'há 3 h');
    expect(fmtAgo(DateTime(2026, 11, 26, 12, 0), now: now), 'há 2 d');
  });

  test('shortHash', () {
    expect(shortHash('9f2c41ab0000000000000000000000000000000000000000000000000071d42ac'), '9f2c41ab…71d42ac');
    expect(shortHash('abc'), 'abc');
  });

  test('tema claro e escuro usam a paleta do design', () {
    final light = buildTheme(Brightness.light);
    expect(light.colorScheme.primary, const Color(0xFF2F5BEA));
    expect(light.scaffoldBackgroundColor, const Color(0xFFF5F6FA));
    final dark = buildTheme(Brightness.dark);
    expect(dark.brightness, Brightness.dark);
    expect(dark.scaffoldBackgroundColor, Pal.dark.bg);
  });
}
