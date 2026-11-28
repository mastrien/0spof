import 'dart:ui' show Tristate;

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:spof_mobile_app/activity_actions.dart';
import 'package:spof_mobile_app/courses_pages.dart';
import 'package:spof_mobile_app/new_delivery_sheet.dart';
import 'package:spof_mobile_app/theme.dart';
import 'package:spof_mobile_app/widgets.dart';

import 'helpers.dart';

void main() {
  group('carimbo RFC 3161', () {
    testWidgets('rota normal (pickAndEnqueue) pede o carimbo por padrão', (tester) async {
      final store = FakeStore();
      late BuildContext ctx;
      await tester.pumpWidget(app(Scaffold(body: Builder(builder: (c) {
        ctx = c;
        return const SizedBox();
      }))));
      await pickAndEnqueue(ctx, store, 9, pick: () async => '/tmp/a.pdf');
      await tester.pump();
      expect(store.calls.single.fetchTsa, isTrue);
      expect(store.calls.single.assignId, 9);
      expect(store.calls.single.path, '/tmp/a.pdf');
    });

    testWidgets('seletor cancelado não enfileira nada', (tester) async {
      final store = FakeStore();
      late BuildContext ctx;
      await tester.pumpWidget(app(Scaffold(body: Builder(builder: (c) {
        ctx = c;
        return const SizedBox();
      }))));
      await pickAndEnqueue(ctx, store, 9, pick: () async => null);
      expect(store.calls, isEmpty);
    });

    Future<FakeStore> openSheet(WidgetTester tester, {String? picked = '/tmp/trabalho.pdf'}) async {
      tester.view.physicalSize = const Size(800, 1600);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final store = FakeStore()..deadlines = [deadline(4, farFuture)];
      await tester.pumpWidget(app(Scaffold(
        body: Builder(
          builder: (c) => TextButton(
            onPressed: () => showNewDeliverySheet(c, store, pick: () async => picked),
            child: const Text('abrir'),
          ),
        ),
      )));
      await tester.tap(find.text('abrir'));
      await tester.pumpAndSettle();
      return store;
    }

    testWidgets('folha: com o interruptor ligado o carimbo chega ao enqueueSubmission', (tester) async {
      final store = await openSheet(tester);
      await tester.tap(find.text('Escolher arquivo'));
      await tester.pumpAndSettle();
      expect(find.text('trabalho.pdf'), findsOneWidget);
      await tester.tap(find.text('Enfileirar com selo criptográfico'));
      await tester.pumpAndSettle();
      expect(store.calls.single.fetchTsa, isTrue);
      expect(store.calls.single.assignId, 4);
      expect(store.calls.single.path, '/tmp/trabalho.pdf');
      expect(find.text('Nova entrega'), findsNothing); // a folha fecha ao terminar
    });

    testWidgets('folha: desligar o interruptor envia fetchTsa=false', (tester) async {
      final store = await openSheet(tester);
      await tester.tap(find.text('Escolher arquivo'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Gerar carimbo de tempo'));
      await tester.pump();
      await tester.tap(find.text('Enfileirar sem carimbo'));
      await tester.pumpAndSettle();
      expect(store.calls.single.fetchTsa, isFalse);
    });
  });

  group('FilterPill: semântica', () {
    testWidgets('botão, selecionado e habilitado', (tester) async {
      final handle = tester.ensureSemantics();
      await tester.pumpWidget(app(Scaffold(
        body: Column(children: [
          FilterPill(label: 'Ligada', selected: true, onTap: () {}),
          FilterPill(label: 'Desligada', selected: false, onTap: () {}),
          const FilterPill(label: 'Inativa', selected: false, onTap: null),
        ]),
      )));
      final on = tester.getSemantics(find.widgetWithText(FilterPill, 'Ligada')).getSemanticsData();
      final off = tester.getSemantics(find.widgetWithText(FilterPill, 'Desligada')).getSemanticsData();
      final dis = tester.getSemantics(find.widgetWithText(FilterPill, 'Inativa')).getSemanticsData();
      expect(on.flagsCollection.isButton, isTrue);
      expect(on.flagsCollection.isSelected, Tristate.isTrue);
      expect(on.flagsCollection.isEnabled, Tristate.isTrue);
      expect(off.flagsCollection.isButton, isTrue);
      expect(off.flagsCollection.isSelected, Tristate.isFalse);
      expect(dis.flagsCollection.isButton, isTrue);
      expect(dis.flagsCollection.isEnabled, Tristate.isFalse);
      expect(on.label, 'Ligada');
      handle.dispose();
    });
  });

  group('SpofCard', () {
    for (final b in Brightness.values) {
      testWidgets('é um Material: ListTile/InkWell filhos pintam o ripple (${b.name})', (tester) async {
        final p = Pal.of(await _ctx(tester, b));
        var taps = 0;
        await tester.pumpWidget(app(
          Scaffold(
            body: SpofCard(child: ListTile(title: const Text('linha'), onTap: () => taps++)),
          ),
          b: b,
        ));
        final nearest = tester.widget<Material>(find.ancestor(of: find.byType(ListTile), matching: find.byType(Material)).first);
        expect(nearest.color, p.surface);
        expect(nearest.shape, isA<RoundedRectangleBorder>());
        await tester.tap(find.text('linha'));
        await tester.pump(const Duration(milliseconds: 100));
        expect(taps, 1);
        // o ripple pinta no Material do cartão (sem DecoratedBox opaco entre os dois)
        expect(tester.takeException(), isNull);
        await tester.pumpAndSettle();
      });
    }
  });

  group('detalhe da disciplina com fonte 2x', () {
    testWidgets('tipo do recurso cresce em vez de vazar', (tester) async {
      tester.view.physicalSize = const Size(800, 4000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final store = FakeStore()
        ..sections = [
          sectionOf(1, 'Álgebra relacional', [
            actJson(1, 'Apostila 03', 'resource', mime: 'application/pdf'),
            actJson(2, 'Questionário 4', 'quiz', due: farFuture),
            actJson(3, 'Fórum de dúvidas', 'forum'),
            actJson(4, 'Entrega 1', 'assign', due: farFuture),
          ]),
        ];
      Future<double> tileWidth(double scale) async {
        await tester.pumpWidget(app(CourseDetailPage(store: store, course: course(1)), scale: scale));
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        final f = find.ancestor(of: find.text('PDF'), matching: find.byType(Container)).first;
        return tester.getSize(f).width;
      }

      final w1 = await tileWidth(1);
      final w2 = await tileWidth(2);
      expect(w1, 36);
      expect(w2, greaterThan(36));
    });
  });
}

Future<BuildContext> _ctx(WidgetTester tester, Brightness b) async {
  late BuildContext ctx;
  await tester.pumpWidget(app(Builder(builder: (c) {
    ctx = c;
    return const SizedBox();
  }), b: b));
  return ctx;
}
