import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:spof_mobile_app/activity_actions.dart';
import 'package:spof_mobile_app/models.dart';
import 'package:spof_mobile_app/new_delivery_sheet.dart';
import 'package:spof_mobile_app/outbox_page.dart';
import 'package:spof_mobile_app/screens.dart';
import 'package:spof_mobile_app/settings_page.dart';
import 'package:spof_mobile_app/theme.dart';

import 'helpers.dart';

void main() {
  group('groupDeadlines', () {
    final now = DateTime(2026, 11, 28, 15, 59);
    test('separa atrasadas, semana, depois, sem data e enviadas vencidas', () {
      final items = [
        deadline(1, '2026-11-27T10:00:00'), // vencida, não enviada
        deadline(2, '2026-11-27T10:00:00', submitted: true), // vencida, enviada
        deadline(3, '2026-11-28T23:00:00'), // hoje
        deadline(4, '2026-12-05T12:00:00'), // +7 dias
        deadline(5, '2026-12-06T12:00:00'), // +8 dias
        deadline(6, null),
        deadline(7, '2026-11-28T16:00:00', submitted: true), // futura e enviada
        deadline(8, null, submitted: true), // sem prazo e enviada
      ];
      final g = groupDeadlines(items, now);
      expect(g.late.map((d) => d.id), [1]);
      // enviadas ficam sempre em "Enviadas", mesmo com prazo futuro ou sem prazo
      expect(g.done.map((d) => d.id), [2, 7, 8]);
      expect(g.week.map((d) => d.id), [3, 4]);
      expect(g.later.map((d) => d.id), [5]);
      expect(g.undated.map((d) => d.id), [6]);
    });

    test('prazo vencido (negativo) nunca cai em "esta semana"', () {
      final g = groupDeadlines([deadline(1, '2026-11-20T10:00:00'), deadline(2, '2026-11-20T10:00:00', submitted: true)], now);
      expect(g.week, isEmpty);
    });
  });

  group('statusBannerFor', () {
    final sync = DateTime(2026, 11, 28, 12, 0);
    final now = DateTime(2026, 11, 28, 12, 12);

    test('verde só com sync feito e nada pendente de proteção', () {
      final b = statusBannerFor(outbox: [outboxItem(1, 'pending', token: true)], pcSyncError: null, pcLastSync: sync, now: now);
      expect(b.tone, Tone.ok);
      expect(b.text, 'Tudo protegido · sync há 12 min');
    });

    test('sync com mais de 24 h é aviso; no limite ainda é verde', () {
      final stale = statusBannerFor(
          outbox: [], pcSyncError: null, pcLastSync: now.subtract(const Duration(days: 5)), now: now);
      expect(stale.tone, Tone.warn);
      expect(stale.text, 'Sync desatualizada · há 5 d');
      final edge = statusBannerFor(
          outbox: [], pcSyncError: null, pcLastSync: now.subtract(const Duration(hours: 24)), now: now);
      expect(edge.tone, Tone.ok);
      final over = statusBannerFor(
          outbox: [], pcSyncError: null, pcLastSync: now.subtract(const Duration(hours: 24, minutes: 1)), now: now);
      expect(over.tone, Tone.warn);
    });

    test('sem nenhuma sync é aviso', () {
      final b = statusBannerFor(outbox: [], pcSyncError: null, pcLastSync: null);
      expect(b.tone, Tone.warn);
      expect(b.text, 'Ainda sem sincronização');
    });

    test('entrega com falha é aviso', () {
      final b = statusBannerFor(outbox: [outboxItem(1, 'failed', token: true)], pcSyncError: null, pcLastSync: sync, now: now);
      expect(b.tone, Tone.warn);
      expect(b.text, startsWith('1 entrega com falha'));
      final c = statusBannerFor(
          outbox: [outboxItem(1, 'failed'), outboxItem(2, 'failed')], pcSyncError: null, pcLastSync: sync, now: now);
      expect(c.text, startsWith('2 entregas com falha'));
    });

    test('sem carimbo é aviso; entregues não contam', () {
      final b = statusBannerFor(outbox: [outboxItem(1, 'pending')], pcSyncError: null, pcLastSync: sync, now: now);
      expect(b.tone, Tone.warn);
      expect(b.text, startsWith('1 entrega sem carimbo'));
      final ok = statusBannerFor(outbox: [outboxItem(1, 'delivered')], pcSyncError: null, pcLastSync: sync, now: now);
      expect(ok.tone, Tone.ok);
    });

    test('só "failed" conta como falha: in_flight e pending com selo seguem verdes', () {
      for (final st in ['in_flight', 'pending']) {
        final b = statusBannerFor(outbox: [outboxItem(1, st, token: true)], pcSyncError: null, pcLastSync: sync, now: now);
        expect(b.tone, Tone.ok, reason: st);
        expect(b.text, 'Tudo protegido · sync há 12 min', reason: st);
      }
    });

    test('erro de sync tem prioridade sobre entregas com falha', () {
      final b = statusBannerFor(
          outbox: [outboxItem(1, 'failed', token: true)], pcSyncError: 'x', pcLastSync: sync, now: now);
      expect(b.text, 'Falha ao sincronizar com o PC');
    });

    test('erro de sync com o PC é aviso e tem prioridade', () {
      final b = statusBannerFor(outbox: [], pcSyncError: 'x', pcLastSync: sync, now: now);
      expect(b.tone, Tone.warn);
      expect(b.text, 'Falha ao sincronizar com o PC');
    });
  });

  group('outboxSteps', () {
    const d = StageState.done, a = StageState.active, n = StageState.pending, e = StageState.error;
    test('estados do backend', () {
      expect(outboxSteps('delivered', true), [d, d, d, d]);
      expect(outboxSteps('delivered', false), [d, d, d, d]);
      expect(outboxSteps('in_flight', true), [d, d, a, n]);
      expect(outboxSteps('in_flight', false), [d, n, a, n]);
      expect(outboxSteps('pending', true), [d, d, n, n]);
      expect(outboxSteps('pending', false), [d, n, n, n]);
      expect(outboxSteps('failed', true), [d, d, e, n]);
      expect(outboxSteps('failed', false), [d, n, e, n]);
    });

    test('rótulos', () {
      expect(outboxStepLabel(outboxSteps('delivered', true)), 'Marcada como enviada');
      expect(outboxStepLabel(outboxSteps('in_flight', true)), 'Enviando');
      expect(outboxStepLabel(outboxSteps('pending', false)), 'Sem selo');
      expect(outboxStepLabel(outboxSteps('in_flight', false)), 'Enviando');
      expect(outboxStepLabel(outboxSteps('pending', true)), 'Aguardando envio');
      expect(outboxStepLabel(outboxSteps('failed', false)), 'Falha em Enviando');
    });
  });

  group('widgets', () {
    testWidgets('enqueueFile respeita o carimbo opcional', (tester) async {
      final store = FakeStore();
      late BuildContext ctx;
      await tester.pumpWidget(app(Scaffold(body: Builder(builder: (c) {
        ctx = c;
        return const SizedBox();
      }))));

      enqueueFile(ctx, store, 3, '/tmp/a.pdf');
      await tester.pump();
      await tester.pump();
      expect(store.calls.last.fetchTsa, isTrue);
      expect(find.text('Entrega #7 na fila, com carimbo RFC 3161.'), findsOneWidget);

      ScaffoldMessenger.of(ctx).clearSnackBars();
      await tester.pump();
      enqueueFile(ctx, store, 4, '/tmp/b.pdf', fetchTsa: false);
      await tester.pump();
      await tester.pump();
      expect(store.calls.last.fetchTsa, isFalse);
      expect(store.calls.last.assignId, 4);
      expect(find.text('Entrega #7 na fila, sem carimbo de tempo.'), findsOneWidget);
    });

    testWidgets('folha Nova entrega alterna o carimbo', (tester) async {
      final store = FakeStore()..deadlines = [deadline(1, farFuture)];
      await tester.pumpWidget(app(Scaffold(body: Builder(
        builder: (c) => TextButton(onPressed: () => showNewDeliverySheet(c, store), child: const Text('abrir')),
      ))));
      await tester.tap(find.text('abrir'));
      await tester.pumpAndSettle();
      expect(find.text('Nova entrega'), findsOneWidget);
      expect(find.text('Registrar entrega'), findsOneWidget);
      await tester.tap(find.text('Gerar carimbo de tempo'));
      await tester.pump();
      expect(find.text('Registrar sem carimbo'), findsOneWidget);
      // sem arquivo escolhido, o botão fica desabilitado
      final btn = tester.widget<FilledButton>(find.byType(FilledButton));
      expect(btn.onPressed, isNull);
    });

    testWidgets('folha Nova entrega avisa quando não há tarefas', (tester) async {
      final store = FakeStore();
      await tester.pumpWidget(app(Scaffold(body: Builder(
        builder: (c) => TextButton(onPressed: () => showNewDeliverySheet(c, store), child: const Text('abrir')),
      ))));
      await tester.tap(find.text('abrir'));
      await tester.pump();
      expect(find.textContaining('Nenhuma tarefa com prazo'), findsOneWidget);
    });

    for (final b in Brightness.values) {
      testWidgets('tela Entregas (${b.name}, fonte 2x) mostra estados sem estourar', (tester) async {
        final store = FakeStore()
          ..deadlines = [deadline(1, farFuture)]
          ..outbox = [
            outboxItem(1, 'pending', token: true),
            outboxItem(2, 'in_flight'),
            outboxItem(3, 'failed', retries: 2),
            outboxItem(4, 'delivered', token: true),
          ];
        tester.view.physicalSize = const Size(800, 6000);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.reset);
        await tester.pumpWidget(app(Scaffold(body: OutboxPage(store: store)), b: b, scale: 2));
        await tester.pumpAndSettle();
        expect(find.text('Entregas'), findsOneWidget);
        expect(find.text('Selo pronto'), findsOneWidget);
        expect(find.text('Enviando'), findsWidgets);
        expect(find.text('Falhou'), findsOneWidget);
        expect(find.text('Marcada como enviada'), findsWidgets);
        expect(find.textContaining('Moodle desconectado'), findsOneWidget);
        expect(tester.takeException(), isNull);
      });

      testWidgets('tela Ajustes (${b.name}, fonte 2x)', (tester) async {
        final store = FakeStore()
          ..version = '0.9.0'
          ..session = SessionInfo.fromJson({'is_authenticated': true, 'base_url': 'https://moodle.test'});
        tester.view.physicalSize = const Size(800, 6000);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.reset);
        await tester.pumpWidget(app(Scaffold(body: SettingsPage(store: store)), b: b, scale: 2));
        await tester.pumpAndSettle();
        expect(find.text('Ajustes'), findsOneWidget);
        expect(find.text('Conectado ao Moodle'), findsOneWidget);
        expect(find.text('moodle.test'), findsOneWidget);
        expect(find.text('Versão 0.9.0'), findsOneWidget);
        expect(find.text('Atualizado'), findsNothing, reason: 'sem checagem ainda, não afirma nada');
        expect(tester.takeException(), isNull);
      });
    }

    testWidgets('casca tem 4 destinos e selo com a fila', (tester) async {
      final store = FakeStore()..outbox = [outboxItem(1, 'pending'), outboxItem(2, 'delivered')];
      await tester.pumpWidget(app(HomeShell(store: store), b: Brightness.dark));
      await tester.pump();
      expect(find.byType(NavigationDestination), findsNWidgets(4));
      expect(find.text('Nova entrega'), findsOneWidget);
      expect(find.text('1'), findsWidgets);
      await tester.tap(find.text('Disciplinas').last);
      await tester.pumpAndSettle();
      expect(find.text('Nova entrega'), findsNothing);
    });
  });
}
