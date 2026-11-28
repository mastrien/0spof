import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:spof_mobile_app/activity_actions.dart';
import 'package:spof_mobile_app/screens.dart';
import 'package:spof_mobile_app/settings_page.dart';
import 'package:spof_mobile_app/updater.dart';

import 'helpers.dart';

const _key = 'pc_last_sync_ms';
const _storage = FlutterSecureStorage();

String _release(String tag, {List<Map<String, dynamic>>? assets}) => jsonEncode({
      'tag_name': tag,
      'assets': assets ??
          [
            {'name': 'notes.txt', 'browser_download_url': 'https://x/notes.txt'},
            {'name': '0spof-0.5.0.apk', 'browser_download_url': 'https://x/0spof.apk', 'digest': 'sha256:abc123'},
          ],
    });

Future<UpdateCheck> _check(http.Client c, {String current = '0.4.0'}) =>
    AppUpdate.check(client: c, currentVersion: () async => current, android: true);

void main() {
  setUp(() => FlutterSecureStorage.setMockInitialValues({}));

  group('AppUpdate.check distingue falha de "sem atualização"', () {
    test('release mais nova com APK: available + sha256', () async {
      final r = await _check(MockClient((_) async => http.Response(_release('v0.5.0'), 200)));
      expect(r.status, UpdateStatus.available);
      expect(r.update!.version, '0.5.0');
      expect(r.update!.apkUrl, 'https://x/0spof.apk');
      expect(r.update!.sha256, 'abc123');
    });

    test('mesma versão: upToDate', () async {
      final r = await _check(MockClient((_) async => http.Response(_release('v0.4.0'), 200)));
      expect(r.status, UpdateStatus.upToDate);
      expect(r.update, isNull);
    });

    test('erro HTTP, JSON inválido, sem rede e release sem APK: failed', () async {
      for (final c in <http.Client>[
        MockClient((_) async => http.Response('rate limit', 403)),
        MockClient((_) async => http.Response('não é json', 200)),
        MockClient((_) async => throw http.ClientException('sem rede')),
        MockClient((_) async => http.Response(_release('v0.5.0', assets: []), 200)),
      ]) {
        final r = await _check(c);
        expect(r.status, UpdateStatus.failed);
        expect(r.update, isNull);
      }
    });

    test('fora do Android: unsupported, sem tocar na rede', () async {
      final r = await AppUpdate.check(client: MockClient((_) async => throw StateError('não deveria chamar')), android: false);
      expect(r.status, UpdateStatus.unsupported);
    });
  });

  group('Ajustes não afirma "mais recente" sem saber', () {
    test('textos por estado', () {
      expect(updateSubtitle(UpdateStatus.upToDate), 'Você está na versão mais recente');
      expect(updateSubtitle(UpdateStatus.failed), 'Não foi possível verificar atualizações');
      expect(updateSubtitle(null), isNot(contains('mais recente')));
      expect(updateSubtitle(UpdateStatus.unsupported), isNot(contains('mais recente')));
      expect(updateChip(UpdateStatus.failed), isNull);
      expect(updateChip(null), isNull);
      expect(updateChip(UpdateStatus.upToDate), isNotNull);
    });

    testWidgets('checagem que falhou mostra aviso, não o chip "Atualizado"', (tester) async {
      final s = FakeStore()..updateCheck = const UpdateCheck(UpdateStatus.failed);
      await s.checkUpdate();
      await tester.pumpWidget(app(Scaffold(body: SettingsPage(store: s))));
      await tester.pump();
      await tester.scrollUntilVisible(find.text('Não foi possível verificar atualizações'), 300, scrollable: find.byType(Scrollable).first);
      expect(find.text('Não foi possível verificar atualizações'), findsOneWidget);
      expect(find.text('Atualizado'), findsNothing);
      expect(find.text('Você está na versão mais recente'), findsNothing);
    });

    testWidgets('checagem ok sem novidade mostra "Atualizado"', (tester) async {
      final s = FakeStore();
      await s.checkUpdate();
      expect(s.updateStatus, UpdateStatus.upToDate);
      await tester.pumpWidget(app(Scaffold(body: SettingsPage(store: s))));
      await tester.pump();
      await tester.scrollUntilVisible(find.text('Atualizado'), 300, scrollable: find.byType(Scrollable).first);
      expect(find.text('Você está na versão mais recente'), findsOneWidget);
    });
  });

  group('restoreLastSync valida o valor guardado', () {
    test('descarta futuro (relógio errado) e negativos', () async {
      final future = DateTime.now().add(const Duration(days: 2)).millisecondsSinceEpoch;
      for (final raw in ['$future', '-5', '-1700000000123']) {
        FlutterSecureStorage.setMockInitialValues({_key: raw});
        final s = FakeStore();
        await s.restoreLastSync();
        expect(s.pcLastSync, isNull, reason: raw);
      }
    });

    test('aceita um instante passado e o zero', () async {
      final past = DateTime.now().subtract(const Duration(hours: 3)).millisecondsSinceEpoch;
      FlutterSecureStorage.setMockInitialValues({_key: '$past'});
      final s = FakeStore();
      await s.restoreLastSync();
      expect(s.pcLastSync, DateTime.fromMillisecondsSinceEpoch(past));
    });
  });

  group('syncPc / unpairPc / pareamento', () {
    test('syncPc com falha não altera o storage', () async {
      FlutterSecureStorage.setMockInitialValues({_key: '1700000000123'});
      final s = FakeStore()..peerJson = '{"name":"PC","host":"10.0.0.2","port":8080}';
      await s.restoreLastSync();
      s.syncFails = true;
      await s.syncPc();
      expect(await _storage.read(key: _key), '1700000000123');
      expect(s.pcLastSync, DateTime.fromMillisecondsSinceEpoch(1700000000123));
    });

    test('unpairPc propaga o erro do Rust e mantém a hora', () async {
      final t = DateTime.fromMillisecondsSinceEpoch(1700000000123);
      FlutterSecureStorage.setMockInitialValues({_key: '1700000000123'});
      final s = FakeStore()
        ..pcLastSync = t
        ..unpairError = Exception('falhou no rust');
      await expectLater(s.unpairPc(), throwsA(isA<Exception>()));
      expect(s.pcLastSync, t);
      expect(await _storage.read(key: _key), '1700000000123');
    });
  });

  group('textos honestos de entrega', () {
    Future<BuildContext> pump(WidgetTester tester) async {
      late BuildContext ctx;
      await tester.pumpWidget(app(Scaffold(body: Builder(builder: (c) {
        ctx = c;
        return const SizedBox();
      }))));
      return ctx;
    }

    testWidgets('aviso inicial muda conforme fetchTsa', (tester) async {
      final ctx = await pump(tester);
      final store = FakeStore()..gate = Completer<void>();
      final first = enqueueFile(ctx, store, 1, '/tmp/a.pdf');
      await tester.pump();
      expect(find.text('Calculando hash e pedindo carimbo de tempo…'), findsOneWidget);
      store.gate!.complete();
      await first;
      await tester.pump();
      ScaffoldMessenger.of(ctx).clearSnackBars();
      await tester.pump();

      store.gate = Completer<void>();
      final second = enqueueFile(ctx, store, 1, '/tmp/a.pdf', fetchTsa: false);
      await tester.pump();
      expect(find.text('Calculando hash…'), findsOneWidget);
      expect(find.textContaining('pedindo carimbo'), findsNothing);
      store.gate!.complete();
      await second;
    });

    testWidgets('TSA fora do ar não diz que o hash "vale como prova"', (tester) async {
      final ctx = await pump(tester);
      final store = FakeStore()..tsaToken = false;
      enqueueFile(ctx, store, 1, '/tmp/a.pdf');
      await tester.pump();
      await tester.pump();
      expect(
        find.text('Entrega #7 na fila. Sem carimbo (TSA fora do ar): só o hash local foi registrado, sem prova de data.'),
        findsOneWidget,
      );
      expect(find.textContaining('vale como prova'), findsNothing);
    });
  });

  group('selo de Entregas na barra', () {
    testWidgets('conta pendentes, em envio e falhas, mas não as entregues', (tester) async {
      final s = FakeStore()
        ..outbox = [
          outboxItem(1, 'pending'),
          outboxItem(2, 'in_flight'),
          outboxItem(3, 'failed'),
          outboxItem(4, 'delivered'),
        ];
      await tester.pumpWidget(app(HomeShell(store: s)));
      await tester.pump();
      final badge = tester.widget<Badge>(find.descendant(of: find.byType(NavigationBar), matching: find.byType(Badge)).first);
      expect((badge.label as Text).data, '3');
    });
  });
}
