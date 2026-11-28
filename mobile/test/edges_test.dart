import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:spof_mobile_app/activity_actions.dart';
import 'package:spof_mobile_app/format.dart';
import 'package:spof_mobile_app/models.dart';
import 'package:spof_mobile_app/outbox_page.dart';
import 'package:spof_mobile_app/settings_page.dart';
import 'package:spof_mobile_app/store.dart';
import 'package:spof_mobile_app/theme.dart';
import 'package:spof_mobile_app/widgets.dart';

import 'helpers.dart';

Future<void> bigView(WidgetTester tester) async {
  tester.view.physicalSize = const Size(800, 6000);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
}

void main() {
  group('fmtCountdown: bordas', () {
    final now = DateTime(2031, 3, 15, 15, 59);
    test('zero à esquerda nos minutos e nas horas', () {
      expect(fmtCountdown(now.add(const Duration(hours: 5, minutes: 5)), now: now), '5h 05min');
      expect(fmtCountdown(now.add(const Duration(days: 1, hours: 3)), now: now), '1d 03h');
    });
    test('exatamente 1 dia', () {
      expect(fmtCountdown(now.add(const Duration(days: 1)), now: now), '1d 00h');
    });
    test('menos de 1 minuto e instante exato', () {
      expect(fmtCountdown(now.add(const Duration(seconds: 30)), now: now), '0h 00min');
      expect(fmtCountdown(now, now: now), '0h 00min');
    });
    test('23h59 ainda é em horas', () {
      expect(fmtCountdown(now.add(const Duration(hours: 23, minutes: 59)), now: now), '23h 59min');
    });
  });

  for (final b in Brightness.values) {
    final p = b == Brightness.dark ? Pal.dark : Pal.light;
    final primary = buildTheme(b).colorScheme.primary;

    group('Entregas (${b.name})', () {
      Future<FakeStore> pumpOutbox(WidgetTester tester, FakeStore store) async {
        await bigView(tester);
        await tester.pumpWidget(app(Scaffold(body: OutboxPage(store: store)), b: b));
        await tester.pump();
        return store;
      }

      Material cardOf(WidgetTester tester, String filename) =>
          tester.widget<Material>(find.ancestor(of: find.text(filename), matching: find.byType(Material)).first);
      Color borderOf(WidgetTester tester, String filename) =>
          (cardOf(tester, filename).shape as RoundedRectangleBorder).side.color;

      testWidgets('chip "Sem selo" é âmbar e os demais tons batem', (tester) async {
        final s = FakeStore()
          ..outbox = [
            outboxItem(1, 'pending'),
            outboxItem(2, 'pending', token: true),
            outboxItem(3, 'failed', token: true),
            outboxItem(4, 'in_flight', token: true),
            outboxItem(5, 'delivered', token: true),
          ];
        await pumpOutbox(tester, s);
        final expected = {
          'Sem selo': p.warnBg,
          'Selo pronto': p.infoBg,
          'Falhou': p.errBg,
          'Enviando': p.infoBg,
          'Confirmado': p.okBg,
        };
        expected.forEach((text, bg) {
          final chip = find.descendant(of: find.widgetWithText(StatusChip, text), matching: find.byType(Container)).first;
          expect((tester.widget<Container>(chip).decoration as BoxDecoration).color, bg, reason: text);
        });
      });

      testWidgets('só o pendente com selo ganha a borda de destaque', (tester) async {
        final s = FakeStore()
          ..outbox = [
            outboxItem(1, 'pending', token: true),
            outboxItem(2, 'failed', token: true),
            outboxItem(3, 'pending'),
            outboxItem(4, 'in_flight', token: true),
            outboxItem(5, 'delivered', token: true),
          ];
        await pumpOutbox(tester, s);
        expect(borderOf(tester, 'arquivo_1.pdf'), primary);
        expect(borderOf(tester, 'arquivo_2.pdf'), p.border, reason: 'failed');
        expect(borderOf(tester, 'arquivo_3.pdf'), p.border, reason: 'sem selo');
        expect(borderOf(tester, 'arquivo_5.pdf'), p.border, reason: 'confirmada');
      });

      testWidgets('tentativas só aparecem com retries > 0', (tester) async {
        final s = FakeStore()
          ..outbox = [outboxItem(1, 'failed', token: true, retries: 2), outboxItem(2, 'pending', token: true)];
        await pumpOutbox(tester, s);
        expect(find.textContaining('Tentativas: 2', findRichText: true), findsOneWidget);
        expect(find.textContaining('Tentativas', findRichText: true), findsOneWidget);
      });

      testWidgets('retries == 1 também mostra', (tester) async {
        final s = FakeStore()..outbox = [outboxItem(1, 'failed', token: true, retries: 1)];
        await pumpOutbox(tester, s);
        expect(find.textContaining('Tentativas: 1', findRichText: true), findsOneWidget);
      });

      testWidgets('rótulo semântico "Etapa atual: ..." em cada estado', (tester) async {
        final handle = tester.ensureSemantics();
        final s = FakeStore()
          ..outbox = [
            outboxItem(1, 'in_flight', token: true),
            outboxItem(2, 'pending'),
            outboxItem(3, 'failed'),
            outboxItem(4, 'pending', token: true),
          ];
        await pumpOutbox(tester, s);
        expect(find.bySemanticsLabel(RegExp('Etapa atual: Enviando')), findsOneWidget);
        expect(find.bySemanticsLabel(RegExp('Etapa atual: Sem selo')), findsOneWidget);
        expect(find.bySemanticsLabel(RegExp('Etapa atual: Falha em Enviando')), findsOneWidget);
        expect(find.bySemanticsLabel(RegExp('Etapa atual: Aguardando envio')), findsOneWidget);
        handle.dispose();
      });

      testWidgets('aviso "Moodle desconectado" só com entrega pendente e sem sessão', (tester) async {
        const aviso = 'Moodle desconectado · entre em Ajustes para enviar';
        // pendente + sem sessão: aparece
        await pumpOutbox(tester, FakeStore()..outbox = [outboxItem(1, 'pending', token: true)]);
        expect(find.text(aviso), findsOneWidget);
        // em andamento ou falha também contam como pendentes
        await pumpOutbox(tester, FakeStore()..outbox = [outboxItem(1, 'failed')]);
        expect(find.text(aviso), findsOneWidget);
        // só entregues: não aparece
        await pumpOutbox(tester, FakeStore()..outbox = [outboxItem(1, 'delivered', token: true)]);
        expect(find.text(aviso), findsNothing);
        // fila vazia: não aparece
        await pumpOutbox(tester, FakeStore());
        expect(find.text(aviso), findsNothing);
        // com sessão: não aparece
        await pumpOutbox(
            tester,
            FakeStore()
              ..outbox = [outboxItem(1, 'pending', token: true)]
              ..session = SessionInfo.fromJson({'is_authenticated': true, 'base_url': 'https://moodle.test'}));
        expect(find.text(aviso), findsNothing);
      });

      testWidgets('título da tarefa e prazo na linha de apoio', (tester) async {
        final s = FakeStore()
          ..deadlines = [deadline(7, '2031-03-17T23:59:00')]
          ..outbox = [outboxItem(1, 'pending', token: true, assignId: 7), outboxItem(2, 'pending', token: true, assignId: 99)];
        await pumpOutbox(tester, s);
        expect(find.text('Tarefa 7 · vence seg, 17/03'), findsOneWidget);
        expect(find.text('Tarefa #99'), findsOneWidget);
      });
    });

    group('Ajustes: subtítulo do pareamento (${b.name})', () {
      Future<void> pumpSettings(WidgetTester tester, FakeStore s) async {
        await bigView(tester);
        await tester.pumpWidget(app(Scaffold(body: SettingsPage(store: s)), b: b));
        await tester.pump();
      }

      testWidgets('sem pareamento, pareado e seus estados', (tester) async {
        await pumpSettings(tester, FakeStore());
        expect(find.text('Parear com o PC'), findsOneWidget);
        expect(find.textContaining('Leia o QR code do app desktop'), findsOneWidget);

        final s = FakeStore()..peer = Peer('Meu PC', '10.0.0.2', 8080);
        await pumpSettings(tester, s);
        expect(find.text('Pareado com Meu PC'), findsOneWidget);
        expect(find.text('10.0.0.2:8080'), findsOneWidget);

        s.pcLastSync = DateTime.now().subtract(const Duration(minutes: 12, seconds: 30));
        await pumpSettings(tester, s);
        expect(find.text('Sincroniza na rede de casa · último sync há 12 min'), findsOneWidget);

        s.pcSyncing = true;
        await pumpSettings(tester, s);
        expect(find.text('Sincronizando…'), findsOneWidget);

        s.pcSyncing = false;
        s.pcSyncError = 'sem rede';
        await pumpSettings(tester, s);
        expect(find.text('sem rede'), findsOneWidget);

        final semNome = FakeStore()..peer = Peer('', 'notebook.local', 1);
        await pumpSettings(tester, semNome);
        expect(find.text('Pareado com notebook.local'), findsOneWidget);
      });
    });
  }

  testWidgets('enqueueFile: falha vira aviso "Falha ao enfileirar: ..."', (tester) async {
    final store = FakeStore()..enqueueError = Exception('boom');
    late BuildContext ctx;
    await tester.pumpWidget(app(Scaffold(body: Builder(builder: (c) {
      ctx = c;
      return const SizedBox();
    }))));
    await enqueueFile(ctx, store, 1, '/tmp/a.pdf');
    await tester.pump();
    expect(find.text('Falha ao enfileirar: Exception: boom'), findsOneWidget);
    expect(store.calls, isEmpty);
  });
}
