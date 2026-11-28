import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:spof_mobile_app/course_export_page.dart';
import 'package:spof_mobile_app/courses_pages.dart';
import 'package:spof_mobile_app/models.dart';
import 'package:spof_mobile_app/outbox_page.dart';
import 'package:spof_mobile_app/screens.dart';
import 'package:spof_mobile_app/settings_page.dart';
import 'package:spof_mobile_app/theme.dart';
import 'package:spof_mobile_app/widgets.dart';

import 'helpers.dart';

const kMinTarget = 44.0;

/// Mede todos os controles interativos da tela atual e exige >= 44 dp nos dois eixos
/// (botões, ícones, FAB e áreas de toque InkWell, que cobrem linhas, cartões e pílulas).
/// InkWells que são parte interna de um botão são ignorados: o botão em si é medido.
void expectTouchTargets(WidgetTester tester, {String where = ''}) {
  final finders = <String, Finder>{
    'InkWell': find.byType(InkWell),
    'IconButton': find.byType(IconButton),
    'botão': find.byWidgetPredicate((w) => w is ButtonStyleButton),
    'FAB': find.byType(FloatingActionButton),
  };
  var measured = 0;
  finders.forEach((kind, f) {
    final elements = f.evaluate().toList();
    for (var i = 0; i < elements.length; i++) {
      final e = elements[i];
      final inner = e.findAncestorWidgetOfExactType<IconButton>() != null ||
          e.findAncestorWidgetOfExactType<FloatingActionButton>() != null ||
          e.findAncestorWidgetOfExactType<FilledButton>() != null ||
          e.findAncestorWidgetOfExactType<OutlinedButton>() != null ||
          e.findAncestorWidgetOfExactType<TextButton>() != null;
      if (kind == 'InkWell' && inner) continue;
      final size = tester.getSize(f.at(i));
      measured++;
      expect(size.height, greaterThanOrEqualTo(kMinTarget), reason: '$where: $kind #$i altura ${size.height}');
      expect(size.width, greaterThanOrEqualTo(kMinTarget), reason: '$where: $kind #$i largura ${size.width}');
    }
  });
  expect(measured, greaterThan(0), reason: '$where: nenhum controle medido');
}

Future<void> big(WidgetTester tester) async {
  tester.view.physicalSize = const Size(800, 6000);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
}

final fixedNow = DateTime(2026, 11, 28, 15, 59);

void main() {
  for (final b in Brightness.values) {
    group('alvos de toque >= 44 dp (${b.name})', () {
      testWidgets('Prazos', (tester) async {
        await big(tester);
        final store = FakeStore()
          ..deadlines = [
            deadline(1, '2026-11-27T10:00:00'),
            deadline(2, '2026-11-30T23:59:00'),
            deadline(3, null),
            deadline(4, '2026-11-29T08:00:00', submitted: true),
          ]
          ..outbox = [outboxItem(1, 'pending', token: true, assignId: 2)];
        await tester.pumpWidget(app(Scaffold(body: DeadlinesPage(store: store, fixedNow: fixedNow)), b: b));
        await tester.pump();
        expectTouchTargets(tester, where: 'Prazos');

        // linha de prazo: pelo menos 64 dp
        final row = find.ancestor(of: find.text('Tarefa 1'), matching: find.byType(InkWell)).first;
        expect(tester.getSize(row).height, greaterThanOrEqualTo(64));
        // bloco de data: pelo menos 44x44
        final tile = tester.getSize(find.ancestor(of: find.text('SEG'), matching: find.byType(Container)).first);
        expect(tile.width, greaterThanOrEqualTo(44));
        expect(tile.height, greaterThanOrEqualTo(44));
        // faixa de status e botão do cabeçalho
        final banner = find.descendant(of: find.byType(InfoBanner), matching: find.byType(Container)).first;
        expect(tester.getSize(banner).height, greaterThanOrEqualTo(44));
        final btn = tester.getSize(find.descendant(of: find.byType(HeaderIconButton), matching: find.byType(IconButton)));
        expect(btn.width, greaterThanOrEqualTo(44));
        expect(btn.height, greaterThanOrEqualTo(44));
        // botão do cartão "Próxima entrega"
        expect(tester.getSize(find.widgetWithText(FilledButton, 'Ver comprovante')).height, greaterThanOrEqualTo(44));
      });

      testWidgets('Disciplinas', (tester) async {
        await big(tester);
        final store = FakeStore()..courses = [course(1), course(2, muted: true, name: 'Redes', short: 'RDC')];
        await tester.pumpWidget(app(Scaffold(body: CoursesPage(store: store)), b: b));
        await tester.pump();
        expectTouchTargets(tester, where: 'Disciplinas');
      });

      testWidgets('Entregas', (tester) async {
        await big(tester);
        final store = FakeStore()
          ..deadlines = [deadline(1, '2026-11-30T23:59:00')]
          ..outbox = [outboxItem(1, 'pending', token: true), outboxItem(2, 'delivered', token: true)];
        await tester.pumpWidget(app(Scaffold(body: OutboxPage(store: store)), b: b));
        await tester.pump();
        expectTouchTargets(tester, where: 'Entregas');
      });

      testWidgets('Ajustes', (tester) async {
        await big(tester);
        final store = FakeStore()
          ..version = '0.9.0'
          ..session = SessionInfo.fromJson({'is_authenticated': true, 'base_url': 'https://moodle.test'});
        await tester.pumpWidget(app(Scaffold(body: SettingsPage(store: store)), b: b));
        await tester.pump();
        expectTouchTargets(tester, where: 'Ajustes');
        // linhas de lista: 56 dp (a de resumo, sem toque, tem 48)
        final rows = find.byType(ListRow);
        for (var i = 0; i < rows.evaluate().length; i++) {
          expect(tester.getSize(rows.at(i)).height, greaterThanOrEqualTo(48), reason: 'ListRow #$i');
        }
        final backup = find.ancestor(of: find.text('Exportar backup'), matching: find.byType(ListRow)).first;
        expect(tester.getSize(backup).height, greaterThanOrEqualTo(56));
      });

      testWidgets('Detalhe da disciplina', (tester) async {
        await big(tester);
        final store = FakeStore()
          ..sections = [
            sectionOf(1, 'Álgebra', [
              actJson(1, 'Apostila', 'resource', mime: 'application/pdf'),
              actJson(2, 'Entrega 1', 'assign', due: relIso(const Duration(days: 2))),
            ]),
          ];
        await tester.pumpWidget(app(CourseDetailPage(store: store, course: course(1)), b: b));
        await tester.pumpAndSettle();
        expectTouchTargets(tester, where: 'Detalhe');
      });

      testWidgets('Exportação', (tester) async {
        await big(tester);
        final store = FakeStore()..preview = exportPreview(withActivity: true);
        await tester.pumpWidget(app(CourseExportPage(store: store, course: course(1)), b: b));
        await tester.pumpAndSettle();
        expectTouchTargets(tester, where: 'Exportação');
      });

      test('botões do tema têm mínimo de 44 dp', () {
        final t = buildTheme(b);
        double h(ButtonStyle? s) => s!.minimumSize!.resolve({})!.height;
        double w(ButtonStyle? s) => s!.minimumSize!.resolve({})!.width;
        for (final s in [t.filledButtonTheme.style, t.outlinedButtonTheme.style, t.textButtonTheme.style]) {
          expect(h(s), greaterThanOrEqualTo(44));
          expect(w(s), greaterThanOrEqualTo(44));
        }
        expect(h(t.filledButtonTheme.style), 48);
      });
    });
  }
}
