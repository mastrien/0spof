import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:spof_mobile_app/screens.dart';
import 'package:spof_mobile_app/theme.dart';

import 'helpers.dart';

/// Sábado, 15/03/2031 15:59 (relógio injetado: nada aqui depende da data real nem do fuso).
final now = DateTime(2031, 3, 15, 15, 59);

Finder hero(Pal p) => find.byWidgetPredicate(
    (w) => w is Container && w.decoration is BoxDecoration && (w.decoration as BoxDecoration).color == p.hero);

FakeStore populated() => FakeStore()
  ..deadlines = [
    deadline(6, '2031-04-05T10:00:00'), // depois
    deadline(7, null), // sem data
    deadline(5, '2031-03-16T08:00:00', submitted: true), // enviada, a mais próxima
    deadline(4, '2031-03-16T10:00:00', type: 'quiz'), // futura mais próxima, mas não é tarefa
    deadline(3, '2031-03-17T23:59:00'), // próxima entrega
    deadline(2, '2031-03-15T13:00:00'), // venceu há 2 h
    deadline(1, '2031-03-14T10:00:00'), // venceu ontem
  ]
  ..outbox = [outboxItem(1, 'pending', token: true, assignId: 3)]
  ..pcLastSync = now.subtract(const Duration(minutes: 12));

Future<void> pumpPage(WidgetTester tester, FakeStore store, {Brightness b = Brightness.light, double scale = 1}) async {
  tester.view.physicalSize = const Size(800, 6000);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(app(Scaffold(body: DeadlinesPage(store: store, fixedNow: now)), b: b, scale: scale));
  await tester.pump();
}

void main() {
  for (final b in Brightness.values) {
    final p = b == Brightness.dark ? Pal.dark : Pal.light;

    testWidgets('grupos na ordem certa, com rótulos (${b.name})', (tester) async {
      await pumpPage(tester, populated(), b: b);
      final labels = ['Atrasadas', 'Esta semana', 'Depois', 'Sem data', 'Enviadas'];
      final ys = [for (final l in labels) tester.getTopLeft(find.text(l)).dy];
      for (var i = 1; i < ys.length; i++) {
        expect(ys[i], greaterThan(ys[i - 1]), reason: '${labels[i]} deve vir depois de ${labels[i - 1]}');
      }
    });

    testWidgets('vencida há poucas horas é "Atrasada", não "Esta semana" (${b.name})', (tester) async {
      await pumpPage(tester, populated(), b: b);
      final late = tester.getTopLeft(find.text('Atrasadas')).dy;
      final week = tester.getTopLeft(find.text('Esta semana')).dy;
      final y2 = tester.getTopLeft(find.text('Tarefa 2')).dy;
      final y1 = tester.getTopLeft(find.text('Tarefa 1')).dy;
      expect(y1, inInclusiveRange(late, week));
      expect(y2, inInclusiveRange(late, week));
    });

    testWidgets('ordem por prazo dentro do grupo (${b.name})', (tester) async {
      await pumpPage(tester, populated(), b: b);
      // atrasadas: a de ontem antes da de 2 h atrás
      expect(tester.getTopLeft(find.text('Tarefa 1')).dy, lessThan(tester.getTopLeft(find.text('Tarefa 2')).dy));
      // semana: quiz (29/11) antes da tarefa 3 (30/11); a linha é a última ocorrência (a primeira é o cartão)
      final rowQuiz = tester.getTopLeft(find.text('Tarefa 4').last).dy;
      final rowAssign = tester.getTopLeft(find.text('Tarefa 3').last).dy;
      expect(rowQuiz, lessThan(rowAssign));
    });

    testWidgets('cartão "Próxima entrega" (${b.name})', (tester) async {
      await pumpPage(tester, populated(), b: b);
      final h = hero(p);
      expect(h, findsOneWidget);
      // prefere tarefa, ignora enviada e atividade que não é tarefa, mesmo mais próximas
      expect(find.descendant(of: h, matching: find.text('Tarefa 3')), findsOneWidget);
      expect(find.descendant(of: h, matching: find.text('Tarefa 5')), findsNothing);
      expect(find.descendant(of: h, matching: find.text('Tarefa 4')), findsNothing);
      expect(find.descendant(of: h, matching: find.text('PRÓXIMA ENTREGA')), findsOneWidget);
      // contagem regressiva medida contra o relógio injetado: 15/03 15:59 -> 17/03 23:59
      expect(find.descendant(of: h, matching: find.textContaining('Vence seg, 17/03 às 23:59 · faltam 2d 08h', findRichText: true)),
          findsOneWidget);
      // entrega na fila com selo
      expect(find.descendant(of: h, matching: find.text('Arquivo na fila')), findsOneWidget);
      expect(find.descendant(of: h, matching: find.text('Selo RFC 3161 pronto')), findsOneWidget);
      expect(find.descendant(of: h, matching: find.text('Ver comprovante')), findsOneWidget);
    });

    testWidgets('sem selo: não anuncia "Selo RFC 3161 pronto" (${b.name})', (tester) async {
      final s = populated()..outbox = [outboxItem(1, 'pending', token: false, assignId: 3)];
      await pumpPage(tester, s, b: b);
      final h = hero(p);
      expect(find.descendant(of: h, matching: find.text('Arquivo na fila')), findsOneWidget);
      expect(find.descendant(of: h, matching: find.text('Selo RFC 3161 pronto')), findsNothing);
    });

    testWidgets('sem entrega na fila: botão enfileira e não há chips (${b.name})', (tester) async {
      final s = populated()..outbox = [];
      await pumpPage(tester, s, b: b);
      final h = hero(p);
      expect(find.descendant(of: h, matching: find.text('Enfileirar entrega')), findsOneWidget);
      expect(find.descendant(of: h, matching: find.text('Arquivo na fila')), findsNothing);
    });

    testWidgets('chips das linhas e faixa de status (${b.name})', (tester) async {
      await pumpPage(tester, populated(), b: b);
      expect(find.text('Atrasado'), findsNWidgets(2));
      expect(find.text('Na fila'), findsOneWidget);
      expect(find.text('Enviado'), findsOneWidget);
      expect(find.text('Sem arquivo'), findsNWidgets(2)); // tarefas 6 e 7
      expect(find.text('Tudo protegido · sync há 12 min'), findsOneWidget);
      expect(find.text('1 na fila'), findsOneWidget);
      expect(find.text('Sábado, 15 de março'.toUpperCase()), findsOneWidget);
    });

    testWidgets('ícone sem data tem rótulo semântico do tipo (${b.name})', (tester) async {
      await pumpPage(tester, populated(), b: b);
      final icon = tester.widget<Icon>(find.byIcon(Icons.assignment_outlined));
      expect(icon.semanticLabel, 'Tarefa');
    });
  }

  testWidgets('estado vazio', (tester) async {
    await pumpPage(tester, FakeStore());
    expect(find.textContaining('Nenhum prazo pela frente'), findsOneWidget);
    expect(find.text('Ainda sem sincronização'), findsOneWidget);
  });
}
