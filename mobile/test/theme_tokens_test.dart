import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:spof_mobile_app/courses_pages.dart';
import 'package:spof_mobile_app/data_actions.dart';
import 'package:spof_mobile_app/models.dart';
import 'package:spof_mobile_app/new_delivery_sheet.dart';
import 'package:spof_mobile_app/settings_page.dart';
import 'package:spof_mobile_app/theme.dart';
import 'package:spof_mobile_app/widgets.dart';

import 'helpers.dart';

/// Valores esperados da paleta (fonte: design aprovado). Se um token mudar, este teste obriga a
/// atualizar a tabela de propósito.
const lightTokens = <String, int>{
  'bg': 0xFFF5F6FA,
  'surface': 0xFFFFFFFF,
  'border': 0xFFE3E7F0,
  'divider': 0xFFEEF1F7,
  'ink': 0xFF14203A,
  'muted': 0xFF5B6783,
  'fieldBorder': 0xFFD5DBE8,
  'softFill': 0xFFF8F9FC,
  'hero': 0xFF14203A,
  'onHero': 0xFFFFFFFF,
  'heroMuted': 0xFFC5D0EA,
  'okBg': 0xFFE3F4EC,
  'okFg': 0xFF146240,
  'warnBg': 0xFFFCF0D8,
  'warnFg': 0xFF7A4B0A,
  'infoBg': 0xFFE6ECFD,
  'infoFg': 0xFF2342B8,
  'mutBg': 0xFFEEF1F7,
  'mutFg': 0xFF4A5672,
  'errBg': 0xFFFBE5E1,
  'errFg': 0xFF9B2C1A,
  'warnCardBg': 0xFFFFFBF1,
  'warnCardBorder': 0xFFF0D9A8,
  'okCardBg': 0xFFF1FAF5,
  'okCardBorder': 0xFFBFE3D0,
  'errBorder': 0xFFEBC4BC,
};

const darkTokens = <String, int>{
  'bg': 0xFF0E1424,
  'surface': 0xFF172038,
  'border': 0xFF2A3452,
  'divider': 0xFF222B45,
  'ink': 0xFFE8ECF6,
  'muted': 0xFFA3AEC8,
  'fieldBorder': 0xFF3A4668,
  'softFill': 0xFF131B30,
  'hero': 0xFF22305A,
  'onHero': 0xFFFFFFFF,
  'heroMuted': 0xFFC5D0EA,
  'okBg': 0xFF14382A,
  'okFg': 0xFF8FE0B8,
  'warnBg': 0xFF3E2F10,
  'warnFg': 0xFFF3CE85,
  'infoBg': 0xFF1E2C5E,
  'infoFg': 0xFFB3C5FF,
  'mutBg': 0xFF222B45,
  'mutFg': 0xFFB4BED6,
  'errBg': 0xFF40201C,
  'errFg': 0xFFF4B1A5,
  'warnCardBg': 0xFF2B2210,
  'warnCardBorder': 0xFF5E4A1E,
  'okCardBg': 0xFF12301F,
  'okCardBorder': 0xFF245A40,
  'errBorder': 0xFF6B3A33,
};

Map<String, Color> tokensOf(Pal p) => {
      'bg': p.bg,
      'surface': p.surface,
      'border': p.border,
      'divider': p.divider,
      'ink': p.ink,
      'muted': p.muted,
      'fieldBorder': p.fieldBorder,
      'softFill': p.softFill,
      'hero': p.hero,
      'onHero': p.onHero,
      'heroMuted': p.heroMuted,
      'okBg': p.okBg,
      'okFg': p.okFg,
      'warnBg': p.warnBg,
      'warnFg': p.warnFg,
      'infoBg': p.infoBg,
      'infoFg': p.infoFg,
      'mutBg': p.mutBg,
      'mutFg': p.mutFg,
      'errBg': p.errBg,
      'errFg': p.errFg,
      'warnCardBg': p.warnCardBg,
      'warnCardBorder': p.warnCardBorder,
      'okCardBg': p.okCardBg,
      'okCardBorder': p.okCardBorder,
      'errBorder': p.errBorder,
    };

Color paint(WidgetTester tester, Finder f) => tester.renderObject<RenderParagraph>(f.first).text.style!.color!;

void main() {
  test('paleta clara e escura: cada token tem o valor aprovado', () {
    final light = tokensOf(Pal.light), dark = tokensOf(Pal.dark);
    expect(light.keys.toSet(), lightTokens.keys.toSet());
    for (final k in lightTokens.keys) {
      expect(light[k]!.toARGB32(), lightTokens[k], reason: 'claro.$k');
      expect(dark[k]!.toARGB32(), darkTokens[k], reason: 'escuro.$k');
    }
    expect(blue, const Color(0xFF2F5BEA));
    expect(navy, const Color(0xFF14203A));
    expect(green, const Color(0xFF1F8A5B));
    expect(amber, const Color(0xFFB45309));
  });

  test('Pal.of escolhe a paleta pelo brilho do tema', () {
    // (verificado via widget abaixo); aqui, os tons mapeiam para os pares certos
    for (final p in [Pal.light, Pal.dark]) {
      expect(p.bgOf(Tone.ok), p.okBg);
      expect(p.fgOf(Tone.ok), p.okFg);
      expect(p.bgOf(Tone.warn), p.warnBg);
      expect(p.fgOf(Tone.warn), p.warnFg);
      expect(p.bgOf(Tone.info), p.infoBg);
      expect(p.fgOf(Tone.info), p.infoFg);
      expect(p.bgOf(Tone.mut), p.mutBg);
      expect(p.fgOf(Tone.mut), p.mutFg);
      expect(p.bgOf(Tone.err), p.errBg);
      expect(p.fgOf(Tone.err), p.errFg);
    }
  });

  for (final b in Brightness.values) {
    final dark = b == Brightness.dark;
    final p = dark ? Pal.dark : Pal.light;
    final t = buildTheme(b);
    final s = t.colorScheme;

    group('tema ${b.name}', () {
      test('cores do ColorScheme e dos componentes', () {
        expect(s.primary.toARGB32(), dark ? 0xFF8CA8FF : 0xFF2F5BEA);
        expect(s.onPrimary.toARGB32(), dark ? 0xFF0E1424 : 0xFFFFFFFF);
        expect(s.surface, p.surface);
        expect(s.onSurface, p.ink);
        expect(s.onSurfaceVariant, p.muted);
        expect(s.outline, p.fieldBorder);
        expect(s.outlineVariant, p.border);
        expect(s.primaryContainer, p.infoBg);
        expect(s.onPrimaryContainer, p.infoFg);
        expect(s.error, p.errFg);
        expect(t.scaffoldBackgroundColor, p.bg);
        expect(t.dividerColor, p.divider);
        expect(t.appBarTheme.backgroundColor, p.surface);
        expect(t.appBarTheme.foregroundColor, p.ink);
        expect((t.cardTheme.shape as RoundedRectangleBorder).side.color, p.border);
        expect(t.cardTheme.color, p.surface);
        expect(t.bottomSheetTheme.backgroundColor, p.surface);
        expect(t.dialogTheme.backgroundColor, p.surface);
        expect(t.inputDecorationTheme.fillColor, p.surface);
        final enabled = t.inputDecorationTheme.enabledBorder as OutlineInputBorder;
        expect(enabled.borderSide.color, p.fieldBorder);
        final focused = t.inputDecorationTheme.focusedBorder as OutlineInputBorder;
        expect(focused.borderSide.color, s.primary);
        expect(t.chipTheme.selectedColor, p.infoBg);
      });

      test('NavigationBar: indicador, ícone e rótulo ativos/inativos', () {
        final n = t.navigationBarTheme;
        expect(n.backgroundColor, p.surface);
        expect(n.indicatorColor, p.infoBg);
        expect(n.iconTheme!.resolve({WidgetState.selected})!.color, p.infoFg);
        expect(n.iconTheme!.resolve({})!.color, p.muted);
        expect(n.labelTextStyle!.resolve({WidgetState.selected})!.color, p.infoFg);
        expect(n.labelTextStyle!.resolve({})!.color, p.ink);
        expect(n.height, 84);
      });

      test('FAB usa a cor de ação com texto legível', () {
        final f = t.floatingActionButtonTheme;
        expect(f.backgroundColor, s.primary);
        expect(f.foregroundColor, s.onPrimary);
        expect(contrastRatio(f.foregroundColor!, f.backgroundColor!), greaterThanOrEqualTo(4.5));
      });

      test('contraste dos pares texto/fundo realmente usados >= 4,5:1', () {
        final pairs = <String, (Color, Color)>{
          'ink/surface': (p.ink, p.surface),
          'ink/bg': (p.ink, p.bg),
          'muted/surface': (p.muted, p.surface),
          'muted/bg': (p.muted, p.bg),
          'muted/mutBg': (p.muted, p.mutBg),
          'muted/softFill': (p.muted, p.softFill),
          'mutFg/bg (caixa do hash)': (p.mutFg, p.bg),
          'mutFg/okCardBg': (p.mutFg, p.okCardBg),
          'ink/okCardBg': (p.ink, p.okCardBg),
          'ink/warnCardBg': (p.ink, p.warnCardBg),
          'warnFg/warnCardBg': (p.warnFg, p.warnCardBg),
          'infoFg/infoBg': (p.infoFg, p.infoBg),
          'infoFg/softFill (Escolher arquivo)': (p.infoFg, p.softFill),
          'infoFg/surface': (p.infoFg, p.surface),
          'errFg/surface': (p.errFg, p.surface),
          'errFg/errBg': (p.errFg, p.errBg),
          'onHero/hero': (p.onHero, p.hero),
          'heroMuted/hero': (p.heroMuted, p.hero),
          'primary/surface': (s.primary, p.surface),
          'primary/bg': (s.primary, p.bg),
          'onPrimary/primary': (s.onPrimary, s.primary),
          'error/bg': (s.error, p.bg),
          'botão Apagar (surface/errFg)': (p.surface, p.errFg),
          'branco/âmbar (badge)': (Colors.white, amber),
          'navy/branco (botão do hero)': (navy, Colors.white),
        };
        pairs.forEach((name, c) {
          expect(contrastRatio(c.$1, c.$2), greaterThanOrEqualTo(4.5), reason: '${b.name}: $name');
        });
        for (final tone in Tone.values) {
          expect(contrastRatio(p.fgOf(tone), p.bgOf(tone)), greaterThanOrEqualTo(4.5), reason: '${b.name}: $tone');
        }
      });

      testWidgets('Pal.of devolve a paleta do brilho do tema', (tester) async {
        late Pal got;
        await tester.pumpWidget(app(Builder(builder: (c) {
          got = Pal.of(c);
          return const SizedBox();
        }), b: b));
        expect(got, same(p));
      });

      testWidgets('StatusChip pinta fundo e texto do tom certo', (tester) async {
        final expected = {
          Tone.ok: (p.okBg, p.okFg),
          Tone.warn: (p.warnBg, p.warnFg),
          Tone.info: (p.infoBg, p.infoFg),
          Tone.mut: (p.mutBg, p.mutFg),
          Tone.err: (p.errBg, p.errFg),
        };
        for (final e in expected.entries) {
          await tester.pumpWidget(app(Scaffold(body: Column(children: [StatusChip('Texto', tone: e.key)])), b: b));
          final box = tester.widget<Container>(find.descendant(of: find.byType(StatusChip), matching: find.byType(Container)).first);
          expect((box.decoration as BoxDecoration).color, e.value.$1, reason: '${e.key} fundo');
          expect(paint(tester, find.text('Texto')), e.value.$2, reason: '${e.key} texto');
        }
      });

      testWidgets('InfoBanner pinta o tom (fundo, ícone, texto)', (tester) async {
        await tester.pumpWidget(app(
            Scaffold(body: Column(children: const [InfoBanner(tone: Tone.warn, icon: Icons.warning, child: Text('aviso'))])),
            b: b));
        final box = tester.widget<Container>(find.descendant(of: find.byType(InfoBanner), matching: find.byType(Container)).first);
        expect((box.decoration as BoxDecoration).color, p.warnBg);
        expect(paint(tester, find.text('aviso')), p.warnFg);
        expect(tester.widget<Icon>(find.byIcon(Icons.warning)).color, p.warnFg);
      });

      testWidgets('Ajustes: cartão de conexão, divisores e área de perigo', (tester) async {
        tester.view.physicalSize = const Size(800, 6000);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.reset);
        final store = FakeStore()
          ..version = '0.9.0'
          ..session = SessionInfo.fromJson({'is_authenticated': true, 'base_url': 'https://moodle.test'});
        await tester.pumpWidget(app(Scaffold(body: SettingsPage(store: store)), b: b));
        await tester.pump();

        Material card(String text) =>
            tester.widget<Material>(find.ancestor(of: find.text(text), matching: find.byType(Material)).first);
        BorderSide side(Material m) => (m.shape as RoundedRectangleBorder).side;

        final ok = card('Conectado ao Moodle');
        expect(ok.color, p.okCardBg);
        expect(side(ok).color, p.okCardBorder);

        final danger = card('Apagar todos os dados locais');
        expect(danger.color, p.surface);
        expect(side(danger).color, p.errBorder);
        expect(paint(tester, find.text('Apagar todos os dados locais')), p.errFg);

        final normal = card('Exportar backup');
        expect(normal.color, p.surface);
        expect(side(normal).color, p.border);

        final dividers = tester.widgetList<Divider>(find.byType(Divider)).toList();
        expect(dividers, isNotEmpty);
        for (final d in dividers) {
          expect(d.color, p.divider);
        }
        expect(find.byIcon(Icons.check), findsOneWidget);
        final dot = tester.widget<Container>(find.ancestor(of: find.byIcon(Icons.check), matching: find.byType(Container)).first);
        expect((dot.decoration as BoxDecoration).color, green);
      });

      testWidgets('diálogo "Apagar": botão em errFg com texto legível', (tester) async {
        final store = FakeStore();
        late BuildContext ctx;
        await tester.pumpWidget(app(Scaffold(body: Builder(builder: (c) {
          ctx = c;
          return const SizedBox();
        })), b: b));
        wipeLocalData(ctx, store);
        await tester.pumpAndSettle();
        final btn = tester.widget<FilledButton>(find.widgetWithText(FilledButton, 'Apagar'));
        final bg = btn.style!.backgroundColor!.resolve({})!;
        final fg = btn.style!.foregroundColor!.resolve({})!;
        expect(bg, p.errFg);
        expect(fg, p.surface);
        expect(contrastRatio(fg, bg), greaterThanOrEqualTo(4.5));
        await tester.tap(find.text('Cancelar'));
        await tester.pumpAndSettle();
      });

      testWidgets('detalhe: cartão do próximo prazo usa o tom âmbar', (tester) async {
        final store = FakeStore()
          ..sections = [
            sectionOf(1, 'Semana 1', [actJson(1, 'Entrega 1', 'assign', due: relIso(const Duration(days: 2)))]),
          ];
        await tester.pumpWidget(app(CourseDetailPage(store: store, course: course(1)), b: b));
        await tester.pumpAndSettle();
        final m = tester.widget<Material>(find.ancestor(of: find.text('Entrega 1'), matching: find.byType(Material)).first);
        expect(m.color, p.warnCardBg);
        expect((m.shape as RoundedRectangleBorder).side.color, p.warnCardBorder);
      });

      testWidgets('folha Nova entrega: zona de arquivo usa softFill e fieldBorder', (tester) async {
        tester.view.physicalSize = const Size(800, 1600);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.reset);
        final store = FakeStore()..deadlines = [deadline(1, relIso(const Duration(days: 2)))];
        await tester.pumpWidget(app(Scaffold(
          body: Builder(builder: (c) => TextButton(onPressed: () => showNewDeliverySheet(c, store), child: const Text('abrir'))),
        ), b: b));
        await tester.tap(find.text('abrir'));
        await tester.pumpAndSettle();
        final zone = tester.widget<Container>(find.ancestor(of: find.text('Escolher arquivo'), matching: find.byType(Container)).first);
        final deco = zone.decoration as BoxDecoration;
        expect(deco.color, p.softFill);
        expect((deco.border as Border).top.color, p.fieldBorder);
        expect(paint(tester, find.text('Escolher arquivo')), p.infoFg);
      });
    });
  }
}
