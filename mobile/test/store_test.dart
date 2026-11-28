import 'package:flutter/material.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:spof_mobile_app/screens.dart';

import 'helpers.dart';

const key = 'pc_last_sync_ms';
const storage = FlutterSecureStorage();

void main() {
  setUp(() => FlutterSecureStorage.setMockInitialValues({}));

  group('última sync persistida', () {
    final t = DateTime.fromMillisecondsSinceEpoch(1700000000123);

    test('restaura um valor válido (em milissegundos)', () async {
      FlutterSecureStorage.setMockInitialValues({key: '1700000000123'});
      final s = FakeStore();
      await s.restoreLastSync();
      expect(s.pcLastSync, t);
    });

    test('valor ausente ou inválido deixa nulo e não lança', () async {
      for (final raw in <Map<String, String>>[{}, {key: 'abc'}, {key: ''}, {key: '12.5'}]) {
        FlutterSecureStorage.setMockInitialValues(raw);
        final s = FakeStore();
        await s.restoreLastSync();
        expect(s.pcLastSync, isNull, reason: '$raw');
      }
    });

    test('grava em milissegundos sob a chave pc_last_sync_ms e restaura em outra instância', () async {
      await FakeStore().saveLastSync(t);
      expect(await storage.read(key: key), '1700000000123');
      final novo = FakeStore();
      await novo.restoreLastSync();
      expect(novo.pcLastSync, t);
    });

    test('syncPc com sucesso grava a hora; com falha mantém a anterior', () async {
      final s = FakeStore()..peerJson = '{"name":"PC","host":"10.0.0.2","port":8080}';
      await s.syncPc();
      expect(s.pcLastSync, isNotNull);
      expect(s.pcSyncError, isNull);
      final raw = await storage.read(key: key);
      expect(int.tryParse(raw ?? ''), s.pcLastSync!.millisecondsSinceEpoch);

      final antes = s.pcLastSync;
      s.syncFails = true;
      await s.syncPc();
      expect(s.pcSyncError, contains('sem rede'));
      expect(s.pcLastSync, antes);
      // modo silencioso não acusa erro
      s.pcSyncError = null;
      await s.syncPc(silent: true);
      expect(s.pcSyncError, isNull);
    });

    test('parear grava a hora (sobrevive a reinício)', () async {
      final s = FakeStore();
      await s.pairWithPc('codigo-qr');
      expect(s.paired, ['codigo-qr']);
      expect(s.pcLastSync, isNotNull);
      expect(s.pcSyncError, isNull);
      expect(int.tryParse(await storage.read(key: key) ?? ''), s.pcLastSync!.millisecondsSinceEpoch);
    });

    test('desparear zera a hora em memória e apaga a chave', () async {
      final s = FakeStore();
      await s.saveLastSync(t);
      s.pcLastSync = t;
      s.pcSyncError = 'x';
      await s.unpairPc();
      expect(s.unpairs, 1);
      expect(s.pcLastSync, isNull);
      expect(s.pcSyncError, isNull);
      expect(await storage.read(key: key), isNull);
      final novo = FakeStore();
      await novo.restoreLastSync();
      expect(novo.pcLastSync, isNull);
    });

    testWidgets('faixa de Prazos volta a "Ainda sem sincronização" após desparear', (tester) async {
      final now = DateTime(2031, 3, 15, 15, 59);
      final s = FakeStore()..pcLastSync = now.subtract(const Duration(minutes: 12));
      await tester.pumpWidget(app(Scaffold(body: DeadlinesPage(store: s, fixedNow: now))));
      await tester.pump();
      expect(find.text('Tudo protegido · sync há 12 min'), findsOneWidget);
      await tester.runAsync(s.unpairPc);
      await tester.pumpWidget(app(Scaffold(body: DeadlinesPage(store: s, fixedNow: now))));
      await tester.pump();
      expect(find.text('Ainda sem sincronização'), findsOneWidget);
      expect(find.textContaining('Tudo protegido'), findsNothing);
    });
  });

  group('HomeShell: relógio de 1 minuto', () {
    testWidgets('a cada minuto o texto relativo é recalculado', (tester) async {
      var t = DateTime(2031, 3, 15, 15, 59);
      final s = FakeStore()..pcLastSync = t.subtract(const Duration(minutes: 12));
      await tester.pumpWidget(app(HomeShell(store: s, clock: () => t)));
      await tester.pump();
      expect(find.text('Tudo protegido · sync há 12 min'), findsOneWidget);

      t = t.add(const Duration(minutes: 1));
      await tester.pump(const Duration(seconds: 30)); // ainda não deu 1 min: nada muda
      expect(find.text('Tudo protegido · sync há 12 min'), findsOneWidget);
      await tester.pump(const Duration(seconds: 30)); // fecha 1 min: o timer reconstrói
      expect(find.text('Tudo protegido · sync há 13 min'), findsOneWidget);

      t = t.add(const Duration(minutes: 1));
      await tester.pump(const Duration(minutes: 1));
      expect(find.text('Tudo protegido · sync há 14 min'), findsOneWidget);
    });

    testWidgets('ao sair da tela o timer é cancelado (sem timers pendentes)', (tester) async {
      final s = FakeStore();
      await tester.pumpWidget(app(HomeShell(store: s)));
      await tester.pumpWidget(app(const SizedBox()));
      await tester.pump(const Duration(minutes: 5));
      // o próprio framework falha o teste se sobrar Timer pendente
      expect(tester.takeException(), isNull);
    });
  });
}
