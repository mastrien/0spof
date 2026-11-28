import 'dart:ui' show Tristate;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:spof_mobile_app/course_export_page.dart';
import 'package:spof_mobile_app/courses_pages.dart';
import 'package:spof_mobile_app/login_page.dart';
import 'package:spof_mobile_app/pair_page.dart';
import 'package:spof_mobile_app/screens.dart';
import 'package:spof_mobile_app/theme.dart';
import 'package:spof_mobile_app/widgets.dart';

import 'helpers.dart';

/// Cor efetivamente pintada pelo primeiro Text encontrado.
Color paintedColor(WidgetTester tester, Finder f) =>
    tester.renderObject<RenderParagraph>(f.first).text.style!.color!;

void main() {
  for (final b in Brightness.values) {
    final p = b == Brightness.dark ? Pal.dark : Pal.light;
    final theme = buildTheme(b);

    group('casca (${b.name})', () {
      testWidgets('badge de Entregas legível, NavigationBar com altura 84 e rótulo ativo azul', (tester) async {
        final store = FakeStore()..outbox = [outboxItem(1, 'pending', token: true)];
        await tester.pumpWidget(app(HomeShell(store: store), b: b));
        await tester.pumpAndSettle();

        // badge: medido no widget real (cor do texto pintado x fundo do Badge)
        final badges = tester
            .widgetList<Badge>(find.byType(Badge))
            .where((x) => x.label != null)
            .toList();
        expect(badges, isNotEmpty);
        final bg = badges.first.backgroundColor!;
        final labels = find.text('1');
        expect(labels, findsWidgets);
        for (var i = 0; i < labels.evaluate().length; i++) {
          final fg = tester.renderObject<RenderParagraph>(labels.at(i)).text.style!.color!;
          expect(contrastRatio(fg, bg), greaterThanOrEqualTo(4.5), reason: 'badge ${b.name} #$i');
        }

        expect(tester.getSize(find.byType(NavigationBar)).height, 84);

        Finder label(String t) => find.descendant(of: find.byType(NavigationBar), matching: find.text(t));
        expect(paintedColor(tester, label('Prazos')), p.infoFg);
        expect(paintedColor(tester, label('Disciplinas')), p.ink);

        await tester.tap(label('Entregas'));
        await tester.pumpAndSettle();
        expect(paintedColor(tester, label('Entregas')), p.infoFg);
        expect(paintedColor(tester, label('Prazos')), p.ink);
      });
    });

    group('erros usam colorScheme.error (${b.name})', () {
      testWidgets('login', (tester) async {
        final store = FakeStore();
        await tester.pumpWidget(app(LoginPage(store: store), b: b));
        await tester.pumpAndSettle();
        await tester.tap(find.text('Continuar'));
        await tester.pump();
        final err = find.text('URL inválida');
        expect(err, findsOneWidget);
        final c = paintedColor(tester, err);
        expect(c, theme.colorScheme.error);
        expect(contrastRatio(c, theme.scaffoldBackgroundColor), greaterThanOrEqualTo(4.5));
      });

      testWidgets('pareamento', (tester) async {
        final store = FakeStore()..pairFails = true;
        await tester.pumpWidget(app(PairPage(store: store), b: b));
        await tester.pump();
        await tester.tap(find.text('Parear'));
        await tester.pump();
        await tester.pump();
        final err = find.textContaining('código inválido');
        expect(err, findsOneWidget);
        final c = paintedColor(tester, err);
        expect(c, theme.colorScheme.error);
        expect(contrastRatio(c, theme.scaffoldBackgroundColor), greaterThanOrEqualTo(4.5));
      });
    });

    testWidgets('disciplina silenciada usa fundo mut, sem Opacity (${b.name})', (tester) async {
      final store = FakeStore()
        ..courses = [course(1, muted: true, name: 'Curso silenciado'), course(2, name: 'Curso ativo')];
      await tester.pumpWidget(app(Scaffold(body: CoursesPage(store: store)), b: b));
      await tester.pump();
      Material cardOf(String name) =>
          tester.widget<Material>(find.ancestor(of: find.text(name), matching: find.byType(Material)).first);
      expect(cardOf('Curso silenciado').color, p.mutBg);
      expect(cardOf('Curso ativo').color, p.surface);
      expect(find.ancestor(of: find.text('Curso silenciado'), matching: find.byType(Opacity)), findsNothing);
      // texto secundário continua legível sobre o fundo mut
      expect(contrastRatio(p.muted, p.mutBg), greaterThanOrEqualTo(4.5));
    });
  }

  group('exportação', () {
    testWidgets('item mostra "arquivo · tamanho" com separador espaçado', (tester) async {
      final store = FakeStore();
      await tester.pumpWidget(app(CourseExportPage(store: store, course: course(1))));
      await tester.pumpAndSettle();
      expect(find.textContaining('aula.pdf · 1,2 MB', findRichText: true), findsOneWidget);
    });

    testWidgets('"Incluir atividades" desabilitada sem atividades', (tester) async {
      final handle = tester.ensureSemantics();
      final store = FakeStore();
      await tester.pumpWidget(app(CourseExportPage(store: store, course: course(1))));
      await tester.pumpAndSettle();
      final pill = find.widgetWithText(FilterPill, 'Incluir atividades');
      expect(tester.widget<FilterPill>(pill).onTap, isNull);
      expect(tester.getSemantics(pill).getSemanticsData().flagsCollection.isEnabled, Tristate.isFalse);
      // as demais continuam habilitadas
      final all = find.widgetWithText(FilterPill, 'Tudo');
      expect(tester.widget<FilterPill>(all).onTap, isNotNull);
      expect(tester.getSemantics(all).getSemanticsData().flagsCollection.isEnabled, Tristate.isTrue);
      handle.dispose();
    });

    testWidgets('"Incluir atividades" habilitada quando há atividades', (tester) async {
      final store = FakeStore()..preview = exportPreview(withActivity: true);
      await tester.pumpWidget(app(CourseExportPage(store: store, course: course(1))));
      await tester.pumpAndSettle();
      final pill = find.widgetWithText(FilterPill, 'Incluir atividades');
      expect(tester.widget<FilterPill>(pill).onTap, isNotNull);
    });
  });

  group('fonte grande: chips crescem em vez de cortar o texto', () {
    Future<Size> sizeOf(WidgetTester tester, Widget w, Finder f, double scale) async {
      await tester.pumpWidget(app(
        Scaffold(body: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [w])),
        scale: scale,
      ));
      await tester.pump();
      return tester.getSize(f);
    }

    testWidgets('StatusChip', (tester) async {
      const chip = StatusChip('Sem selo');
      final h1 = (await sizeOf(tester, chip, find.byType(StatusChip), 1)).height;
      final h2 = (await sizeOf(tester, chip, find.byType(StatusChip), 2)).height;
      expect(h1, 24);
      expect(h2, greaterThan(h1));
    });

    testWidgets('FilterPill', (tester) async {
      final pill = FilterPill(label: 'Materiais', selected: false, onTap: () {});
      Finder inner() => find.descendant(of: find.byType(FilterPill), matching: find.byType(Container)).first;
      final s1 = await sizeOf(tester, pill, inner(), 1);
      final o1 = tester.getSize(find.byType(FilterPill));
      final s3 = await sizeOf(tester, pill, inner(), 3);
      final o3 = tester.getSize(find.byType(FilterPill));
      expect(s1.height, 36);
      expect(o1.height, 44);
      expect(s3.height, greaterThan(36));
      expect(o3.height, greaterThan(44));
    });

    testWidgets('chip do cartão "Próxima entrega"', (tester) async {
      const due = '2031-03-17T23:59:00';
      final store = FakeStore()
        ..deadlines = [deadline(1, due)]
        ..outbox = [outboxItem(1, 'pending', token: true)];
      Future<double> heroChipHeight(double scale) async {
        await tester.pumpWidget(app(Scaffold(body: DeadlinesPage(store: store, fixedNow: fixedClock)), scale: scale));
        await tester.pump();
        final f = find.ancestor(of: find.text('Arquivo na fila'), matching: find.byType(Container)).first;
        return tester.getSize(f).height;
      }

      final h1 = await heroChipHeight(1);
      final h2 = await heroChipHeight(2);
      expect(h1, 24);
      expect(h2, greaterThan(h1));
    });
  });
}
