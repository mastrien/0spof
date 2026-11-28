import 'dart:async';

import 'package:flutter/material.dart';

import 'activity_actions.dart';
import 'courses_pages.dart';
import 'format.dart';
import 'models.dart';
import 'new_delivery_sheet.dart';
import 'outbox_page.dart';
import 'settings_page.dart';
import 'store.dart';
import 'theme.dart';
import 'widgets.dart';

/// Espaço no fim das listas para o botão "Nova entrega" não cobrir o último item.
const kFabClearance = 96.0;

/// Estado vazio com ícone e texto, rolável para permitir o puxar-para-atualizar.
Widget emptyState(IconData icon, String text) => Builder(
      builder: (context) {
        final p = Pal.of(context);
        return Column(
          children: [
            const SizedBox(height: 72),
            Icon(icon, size: 48, color: p.muted),
            const SizedBox(height: 12),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 32),
              child: Text(text, textAlign: TextAlign.center, style: TextStyle(color: p.muted)),
            ),
          ],
        );
      },
    );

/// Lista rolável de página com puxar-para-atualizar (sync com o PC + recarga do banco).
Widget refreshableList(Store store, List<Widget> children) => RefreshIndicator(
      onRefresh: () async {
        await store.syncPc();
        await store.refresh();
      },
      child: ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.only(bottom: kFabClearance),
        children: children,
      ),
    );

/// Cartão com linhas separadas por filete.
Widget dividedCard(BuildContext context, List<Widget> rows, {EdgeInsetsGeometry margin = const EdgeInsets.symmetric(horizontal: 16)}) {
  final p = Pal.of(context);
  return SpofCard(
    margin: margin,
    child: Column(
      children: [
        for (var i = 0; i < rows.length; i++) ...[
          if (i > 0) Divider(height: 1, thickness: 1, color: p.divider),
          rows[i],
        ],
      ],
    ),
  );
}

class HomeShell extends StatefulWidget {
  final Store store;
  const HomeShell({super.key, required this.store});
  @override
  State<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends State<HomeShell> {
  int _tab = 0;
  Timer? _tick;

  @override
  void initState() {
    super.initState();
    // Reconstrói a cada minuto para os textos relativos ("sync há 12 min", contagem regressiva) não ficarem velhos.
    _tick = Timer.periodic(const Duration(minutes: 1), (_) {
      if (mounted) setState(() {});
    });
  }

  @override
  void dispose() {
    _tick?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final s = widget.store;
    return ListenableBuilder(
      listenable: s,
      builder: (context, _) {
        final pages = [
          DeadlinesPage(store: s),
          CoursesPage(store: s),
          OutboxPage(store: s),
          SettingsPage(store: s),
        ];
        final busy = s.syncing || s.pcSyncing;
        final pending = s.outbox.where((o) => o.status != 'delivered').length;
        return Scaffold(
          body: SafeArea(
            bottom: false,
            child: Column(
              children: [
                if (busy) const LinearProgressIndicator(minHeight: 2),
                Expanded(child: pages[_tab]),
              ],
            ),
          ),
          floatingActionButton: (_tab == 0 || _tab == 2)
              ? FloatingActionButton.extended(
                  onPressed: () => showNewDeliverySheet(context, s),
                  icon: const Icon(Icons.add),
                  label: const Text('Nova entrega'),
                )
              : null,
          bottomNavigationBar: NavigationBar(
            selectedIndex: _tab,
            onDestinationSelected: (i) => setState(() => _tab = i),
            destinations: [
              const NavigationDestination(
                icon: Icon(Icons.calendar_today_outlined),
                selectedIcon: Icon(Icons.calendar_today),
                label: 'Prazos',
              ),
              const NavigationDestination(
                icon: Icon(Icons.menu_book_outlined),
                selectedIcon: Icon(Icons.menu_book),
                label: 'Disciplinas',
              ),
              NavigationDestination(
                icon: Badge(
                  isLabelVisible: pending > 0,
                  backgroundColor: amber,
                  textColor: Colors.white,
                  label: Text('$pending'),
                  child: const Icon(Icons.send_outlined),
                ),
                selectedIcon: Badge(
                  isLabelVisible: pending > 0,
                  backgroundColor: amber,
                  textColor: Colors.white,
                  label: Text('$pending'),
                  child: const Icon(Icons.send),
                ),
                label: 'Entregas',
              ),
              NavigationDestination(
                icon: Badge(isLabelVisible: s.update != null, smallSize: 8, child: const Icon(Icons.tune)),
                selectedIcon: Badge(isLabelVisible: s.update != null, smallSize: 8, child: const Icon(Icons.tune)),
                label: 'Ajustes',
              ),
            ],
          ),
        );
      },
    );
  }
}

/// Entrega ainda não confirmada para a tarefa `assignId`, se houver.
OutboxItem? queuedFor(Store store, int assignId) {
  for (final o in store.outbox) {
    if (o.assignId == assignId && o.status != 'delivered') return o;
  }
  return null;
}

/// Prazos agrupados para a tela Prazos.
class DeadlineGroups {
  final List<Deadline> late = [], week = [], later = [], undated = [], done = [];
}

/// Tarefas já enviadas vão sempre para "Enviadas" (com ou sem prazo, vencido ou futuro),
/// coerente com o rótulo. As demais: atrasadas (vencidas), esta semana (hoje até 7 dias
/// à frente), depois e sem data.
DeadlineGroups groupDeadlines(List<Deadline> items, DateTime now) {
  final g = DeadlineGroups();
  for (final d in items) {
    final due = d.due;
    if (d.submitted) {
      g.done.add(d);
    } else if (due == null) {
      g.undated.add(d);
    } else if (due.isBefore(now)) {
      g.late.add(d);
    } else if (daysUntil(due, now: now) <= 7) {
      g.week.add(d);
    } else {
      g.later.add(d);
    }
  }
  return g;
}

/// Idade a partir da qual a última sincronização é considerada desatualizada.
const staleSyncAfter = Duration(hours: 24);

/// Texto, tom e ícone da faixa de status de Prazos.
({String text, Tone tone, IconData icon}) statusBannerFor({
  required List<OutboxItem> outbox,
  required String? pcSyncError,
  required DateTime? pcLastSync,
  DateTime? now,
}) {
  final open = outbox.where((o) => o.status != 'delivered');
  final failed = open.where((o) => o.status == 'failed').length;
  final unprotected = open.where((o) => o.status != 'failed' && !o.hasToken).length;
  final syncText = pcLastSync == null ? '' : ' · sync ${fmtAgo(pcLastSync, now: now)}';
  const warn = Icons.warning_amber_rounded;
  if (pcSyncError != null) {
    return (text: 'Falha ao sincronizar com o PC', tone: Tone.warn, icon: warn);
  }
  if (failed > 0) {
    return (
      text: '$failed ${failed == 1 ? 'entrega com falha' : 'entregas com falha'}$syncText',
      tone: Tone.warn,
      icon: warn
    );
  }
  if (unprotected > 0) {
    return (
      text: '$unprotected ${unprotected == 1 ? 'entrega sem carimbo' : 'entregas sem carimbo'}$syncText',
      tone: Tone.warn,
      icon: warn
    );
  }
  if (pcLastSync == null) {
    return (text: 'Ainda sem sincronização', tone: Tone.warn, icon: warn);
  }
  // Sync antiga (mais de 24 h) não é "tudo protegido": os dados podem estar defasados.
  if ((now ?? DateTime.now()).difference(pcLastSync) > staleSyncAfter) {
    return (text: 'Sync desatualizada · ${fmtAgo(pcLastSync, now: now)}', tone: Tone.warn, icon: warn);
  }
  return (text: 'Tudo protegido$syncText', tone: Tone.ok, icon: Icons.verified_user_outlined);
}

class DeadlinesPage extends StatelessWidget {
  final Store store;
  /// `fixedNow` só existe para testes (relógio injetado); em produção vale `DateTime.now()`.
  final DateTime? fixedNow;
  const DeadlinesPage({super.key, required this.store, this.fixedNow});

  Deadline? _next(List<Deadline> items, DateTime now) {
    Deadline? any;
    for (final d in items) {
      if (d.submitted || d.due == null || !d.due!.isAfter(now)) continue;
      if (d.type == 'assign') return d;
      any ??= d;
    }
    return any;
  }

  @override
  Widget build(BuildContext context) {
    final now = fixedNow ?? DateTime.now();
    final items = [...store.deadlines]..sort((a, b) {
        if (a.due == null && b.due == null) return 0;
        if (a.due == null) return 1;
        if (b.due == null) return -1;
        return a.due!.compareTo(b.due!);
      });
    final next = _next(items, now);
    final g = groupDeadlines(items, now);

    final children = <Widget>[
      ScreenHeader(
        eyebrow: fmtToday(now),
        title: 'Prazos',
        trailing: HeaderIconButton(
          tooltip: 'Sincronizar agora',
          icon: Icons.sync,
          onPressed: store.pcSyncing || store.syncing
              ? null
              : () async {
                  await store.syncPc();
                  await store.refresh();
                },
        ),
      ),
      _StatusBanner(store: store, now: now),
    ];

    if (items.isEmpty) {
      children.add(emptyState(Icons.event_available, 'Nenhum prazo pela frente.\nSincronize para trazer as atividades.'));
    } else {
      if (next != null) children.add(_NextCard(store: store, d: next, now: now));
      void group(String label, List<Deadline> list) {
        if (list.isEmpty) return;
        children.add(GroupLabel(label, padding: const EdgeInsets.fromLTRB(20, 16, 20, 8)));
        children.add(dividedCard(context, [for (final d in list) _DeadlineRow(store: store, d: d, now: now)]));
      }

      group('Atrasadas', g.late);
      group('Esta semana', g.week);
      group('Depois', g.later);
      group('Sem data', g.undated);
      group('Enviadas', g.done);
    }
    return refreshableList(store, children);
  }
}

class _StatusBanner extends StatelessWidget {
  final Store store;
  final DateTime now;
  const _StatusBanner({required this.store, required this.now});

  @override
  Widget build(BuildContext context) {
    final pending = store.outbox.where((o) => o.status != 'delivered').length;
    final b = statusBannerFor(
      outbox: store.outbox,
      pcSyncError: store.pcSyncError,
      pcLastSync: store.pcLastSync,
      now: now,
    );
    return InfoBanner(
      tone: b.tone,
      icon: b.icon,
      child: Row(
        children: [
          Expanded(child: Text(b.text)),
          if (pending > 0) Text('$pending na fila', style: const TextStyle(fontWeight: FontWeight.w500)),
        ],
      ),
    );
  }
}

class _HeroChip extends StatelessWidget {
  final String text;
  const _HeroChip(this.text);

  @override
  Widget build(BuildContext context) => Container(
        constraints: const BoxConstraints(minHeight: 24),
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 2),
        alignment: Alignment.center,
        decoration: BoxDecoration(color: Colors.white.withValues(alpha: 0.14), borderRadius: BorderRadius.circular(999)),
        child: Text(text, style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w600, color: Colors.white)),
      );
}

class _NextCard extends StatelessWidget {
  final Store store;
  final Deadline d;
  final DateTime now;
  const _NextCard({required this.store, required this.d, required this.now});

  @override
  Widget build(BuildContext context) {
    final p = Pal.of(context);
    final due = d.due!;
    final queued = d.type == 'assign' ? queuedFor(store, d.id) : null;
    final String label;
    final VoidCallback action;
    if (queued != null) {
      label = 'Ver comprovante';
      action = () => showProof(context, store, queued.id);
    } else if (d.type == 'assign') {
      label = 'Enfileirar entrega';
      action = () => pickAndEnqueue(context, store, d.id);
    } else {
      label = 'Abrir';
      action = () => showActivityActions(context, store, id: d.id, title: d.title, type: d.type, url: d.url);
    }
    return Container(
      margin: const EdgeInsets.fromLTRB(16, 14, 16, 0),
      padding: const EdgeInsets.all(18),
      decoration: BoxDecoration(color: p.hero, borderRadius: BorderRadius.circular(16)),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Eyebrow('Próxima entrega', color: Color(0xFFA9B8DC)),
          const SizedBox(height: 4),
          Text(
            d.title,
            maxLines: 3,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700, letterSpacing: -0.2, height: 1.25, color: p.onHero),
          ),
          const SizedBox(height: 4),
          Text.rich(
            TextSpan(
              style: TextStyle(fontSize: 13, color: p.heroMuted),
              children: [
                TextSpan(text: 'Vence ${fmtDueLong(due)} · faltam '),
                TextSpan(text: fmtCountdown(due, now: now), style: TextStyle(fontWeight: FontWeight.w700, color: p.onHero)),
              ],
            ),
          ),
          if (queued != null) ...[
            const SizedBox(height: 12),
            Wrap(
              spacing: 6,
              runSpacing: 6,
              children: [
                const _HeroChip('Arquivo na fila'),
                if (queued.hasToken) const _HeroChip('Selo RFC 3161 pronto'),
              ],
            ),
          ],
          const SizedBox(height: 14),
          SizedBox(
            width: double.infinity,
            child: FilledButton(
              style: FilledButton.styleFrom(
                backgroundColor: Colors.white,
                foregroundColor: navy,
                minimumSize: const Size.fromHeight(48),
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
              ),
              onPressed: action,
              child: Text(label),
            ),
          ),
        ],
      ),
    );
  }
}

class _DeadlineRow extends StatelessWidget {
  final Store store;
  final Deadline d;
  final DateTime now;
  const _DeadlineRow({required this.store, required this.d, required this.now});

  @override
  Widget build(BuildContext context) {
    final p = Pal.of(context);
    final due = d.due;
    final late = due != null && due.isBefore(now) && !d.submitted;
    final queued = d.type == 'assign' ? queuedFor(store, d.id) : null;
    final Widget? chip = d.submitted
        ? const StatusChip('Enviado', tone: Tone.ok)
        : queued != null
            ? const StatusChip('Na fila', tone: Tone.info)
            : late
                ? const StatusChip('Atrasado', tone: Tone.err)
                : d.type == 'assign'
                    ? const StatusChip('Sem arquivo', tone: Tone.warn)
                    : null;
    final hm = due == null ? null : '${due.hour.toString().padLeft(2, '0')}:${due.minute.toString().padLeft(2, '0')}';
    return InkWell(
      onTap: () => showActivityActions(context, store, id: d.id, title: d.title, type: d.type, url: d.url),
      child: Container(
        constraints: const BoxConstraints(minHeight: 64),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
        child: Row(
          children: [
            Container(
              // cresce com fonte grande em vez de estourar
              constraints: const BoxConstraints(minWidth: 44, minHeight: 44),
              padding: const EdgeInsets.symmetric(horizontal: 2, vertical: 2),
              alignment: Alignment.center,
              decoration: BoxDecoration(color: p.mutBg, borderRadius: BorderRadius.circular(10)),
              child: due == null
                  ? Icon(typeIcon(d.type), color: p.muted, semanticLabel: typeLabel(d.type))
                  : Column(
                      mainAxisSize: MainAxisSize.min,
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Text(weekdayAbbr(due),
                            style: TextStyle(fontSize: 10, fontWeight: FontWeight.w600, letterSpacing: 0.6, color: p.muted)),
                        Text(due.day.toString().padLeft(2, '0'),
                            style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, height: 1.1, color: p.ink)),
                      ],
                    ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(d.title,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: p.ink)),
                  Text(
                    [typeLabel(d.type), ?hm].join(' · '),
                    style: TextStyle(fontSize: 12, color: p.muted),
                  ),
                ],
              ),
            ),
            if (chip != null) ...[const SizedBox(width: 8), chip],
          ],
        ),
      ),
    );
  }
}
