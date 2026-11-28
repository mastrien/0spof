import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:spof_mobile_app/format.dart';
import 'package:spof_mobile_app/theme.dart';

void main() {
  final now = DateTime(2026, 11, 28, 15, 59);

  test('fmtToday em português', () {
    expect(fmtToday(DateTime(2026, 11, 28)), 'Sábado, 28 de novembro');
  });

  test('datas curtas', () {
    final d = DateTime(2026, 11, 30, 23, 59);
    expect(weekdayAbbr(d), 'SEG');
    expect(fmtShortDate(d), 'seg, 30/11');
    expect(fmtDueLong(d), 'seg, 30/11 às 23:59');
  });

  test('daysUntil conta dias de calendário, não horas', () {
    expect(daysUntil(DateTime(2026, 11, 28, 0, 1), now: now), 0);
    expect(daysUntil(DateTime(2026, 11, 29, 0, 1), now: now), 1);
    expect(daysUntil(DateTime(2026, 12, 5, 23, 0), now: now), 7);
    expect(daysUntil(DateTime(2026, 11, 27, 23, 0), now: now), -1);
    expect(daysUntil(DateTime(2026, 11, 20), now: now), -8);
  });

  test('fmtDue', () {
    expect(fmtDue(DateTime(2026, 11, 28, 23, 0), now: now), '28/11 23:00 · hoje');
    expect(fmtDue(DateTime(2026, 11, 29, 8, 5), now: now), '29/11 08:05 · amanhã');
    expect(fmtDue(DateTime(2026, 12, 3, 12, 0), now: now), '03/12 12:00 · em 5 dias');
    expect(fmtDue(DateTime(2026, 11, 25, 12, 0), now: now), '25/11 12:00 · passou há 3 d');
  });

  test('fmtDaysChip', () {
    expect(fmtDaysChip(DateTime(2026, 11, 27, 23), now: now), 'Atrasado');
    expect(fmtDaysChip(DateTime(2026, 11, 28, 23), now: now), 'Hoje');
    expect(fmtDaysChip(DateTime(2026, 11, 29, 23), now: now), 'Amanhã');
    expect(fmtDaysChip(DateTime(2026, 12, 3, 23), now: now), '5 dias');
  });

  test('fmtCountdown', () {
    expect(fmtCountdown(DateTime(2026, 11, 30, 23, 59), now: now), '2d 08h');
    expect(fmtCountdown(DateTime(2026, 11, 28, 21, 11), now: now), '5h 12min');
    expect(fmtCountdown(DateTime(2026, 11, 28, 10, 0), now: now), 'venceu');
  });

  test('fmtAgo e seus limites', () {
    final n = DateTime(2026, 11, 28, 12, 0);
    expect(fmtAgo(DateTime(2026, 11, 28, 11, 59, 30), now: n), 'agora');
    expect(fmtAgo(DateTime(2026, 11, 28, 11, 59), now: n), 'há 1 min');
    expect(fmtAgo(DateTime(2026, 11, 28, 11, 48), now: n), 'há 12 min');
    expect(fmtAgo(DateTime(2026, 11, 28, 11, 1), now: n), 'há 59 min');
    expect(fmtAgo(DateTime(2026, 11, 28, 11, 0), now: n), 'há 1 h');
    expect(fmtAgo(DateTime(2026, 11, 28, 9, 0), now: n), 'há 3 h');
    expect(fmtAgo(DateTime(2026, 11, 27, 12, 0), now: n), 'há 1 d');
    expect(fmtAgo(DateTime(2026, 11, 26, 12, 0), now: n), 'há 2 d');
  });

  test('shortHash', () {
    expect(shortHash('9f2c41ab0000000000000000000000000000000000000000000000000071d42ac'), '9f2c41ab…71d42ac');
    expect(shortHash('abc'), 'abc');
  });

  test('typeLabel e typeIcon', () {
    expect(typeLabel('assign'), 'Tarefa');
    expect(typeLabel('resource'), 'Arquivo');
    expect(typeLabel('folder'), 'Pasta');
    expect(typeLabel('url'), 'Link');
    expect(typeLabel('forum'), 'Fórum');
    expect(typeLabel('quiz'), 'Questionário');
    expect(typeLabel('page'), 'Página');
    expect(typeLabel('outro'), 'outro');
    expect(typeIcon('assign'), Icons.assignment_outlined);
    expect(typeIcon('resource'), Icons.description_outlined);
    expect(typeIcon('folder'), Icons.folder_outlined);
    expect(typeIcon('url'), Icons.link);
    expect(typeIcon('forum'), Icons.forum_outlined);
    expect(typeIcon('quiz'), Icons.quiz_outlined);
    expect(typeIcon('page'), Icons.extension_outlined);
    expect(typeIcon('xyz'), Icons.extension_outlined);
  });

  test('tema claro e escuro usam a paleta do design', () {
    final light = buildTheme(Brightness.light);
    expect(light.colorScheme.primary, const Color(0xFF2F5BEA));
    expect(light.colorScheme.onPrimary, Colors.white);
    expect(light.scaffoldBackgroundColor, const Color(0xFFF5F6FA));
    final dark = buildTheme(Brightness.dark);
    expect(dark.brightness, Brightness.dark);
    expect(dark.colorScheme.primary, const Color(0xFF8CA8FF));
    expect(dark.colorScheme.onPrimary, const Color(0xFF0E1424));
    expect(dark.scaffoldBackgroundColor, Pal.dark.bg);
  });

  double contrast(Color a, Color b) {
    final la = a.computeLuminance(), lb = b.computeLuminance();
    final hi = la > lb ? la : lb, lo = la > lb ? lb : la;
    return (hi + 0.05) / (lo + 0.05);
  }

  test('contraste mínimo 4,5:1 em pares críticos (claro e escuro)', () {
    for (final p in [Pal.light, Pal.dark]) {
      expect(contrast(p.ink, p.surface), greaterThanOrEqualTo(4.5));
      expect(contrast(p.muted, p.surface), greaterThanOrEqualTo(4.5));
      expect(contrast(p.muted, p.mutBg), greaterThanOrEqualTo(4.5));
      for (final t in Tone.values) {
        expect(contrast(p.fgOf(t), p.bgOf(t)), greaterThanOrEqualTo(4.5), reason: '$t');
      }
    }
    expect(contrast(Colors.white, amber), greaterThanOrEqualTo(4.5));
  });
}
