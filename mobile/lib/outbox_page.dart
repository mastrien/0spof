import 'package:flutter/material.dart';

import 'activity_actions.dart';
import 'format.dart';
import 'models.dart';
import 'screens.dart';
import 'store.dart';
import 'theme.dart';
import 'widgets.dart';

/// Estado de cada etapa do comprovante.
enum StageState { pending, done, active, error }

const outboxStepNames = ['Registrado', 'Selo', 'Enviando', 'Confirmado'];

/// Mapeia o status do backend (`pending`, `in_flight`, `delivered`, `failed`)
/// e a presença do selo RFC 3161 nas 4 etapas [Registrado, Selo, Enviando, Confirmado].
List<StageState> outboxSteps(String status, bool hasToken) {
  const d = StageState.done, a = StageState.active, n = StageState.pending, e = StageState.error;
  final selo = hasToken ? d : n;
  return switch (status) {
    'delivered' => [d, d, d, d],
    'failed' => [d, selo, e, n],
    'in_flight' => [d, selo, a, n],
    // pendente: com selo, a próxima etapa é o envio (ainda não iniciado); sem selo, o selo é a etapa atual
    _ => hasToken ? [d, d, n, n] : [d, a, n, n],
  };
}

/// Rótulo da etapa atual, para leitores de tela.
String outboxStepLabel(List<StageState> s) {
  if (s.every((x) => x == StageState.done)) return 'Confirmado';
  final err = s.indexOf(StageState.error);
  if (err >= 0) return 'Falha em ${outboxStepNames[err]}';
  final act = s.indexOf(StageState.active);
  if (act >= 0) return outboxStepNames[act];
  return 'Aguardando envio';
}

class OutboxPage extends StatelessWidget {
  final Store store;
  const OutboxPage({super.key, required this.store});

  Deadline? _deadlineOf(int assignId) {
    for (final d in store.deadlines) {
      if (d.id == assignId) return d;
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final items = store.outbox;
    final hasPending = items.any((o) => o.status != 'delivered');
    final children = <Widget>[
      const ScreenHeader(eyebrow: 'Envios protegidos', title: 'Entregas'),
      if (hasPending && !store.session.authenticated)
        const InfoBanner(
          tone: Tone.warn,
          icon: Icons.cloud_off_outlined,
          child: Text('Moodle desconectado · entre em Ajustes para enviar'),
        )
      else
        const SizedBox(height: 14),
    ];
    if (items.isEmpty) {
      children.add(emptyState(Icons.outbox_outlined, 'Nenhuma entrega na fila.'));
    } else {
      for (final o in items) {
        children.add(_OutboxCard(store: store, o: o, deadline: _deadlineOf(o.assignId)));
      }
    }
    return refreshableList(store, children);
  }
}

class _OutboxCard extends StatelessWidget {
  final Store store;
  final OutboxItem o;
  final Deadline? deadline;
  const _OutboxCard({required this.store, required this.o, required this.deadline});

  @override
  Widget build(BuildContext context) {
    final p = Pal.of(context);
    final delivered = o.status == 'delivered';
    final failed = o.status == 'failed';
    final due = deadline?.due;

    final String chipText;
    final Tone tone;
    if (delivered) {
      chipText = 'Confirmado';
      tone = Tone.ok;
    } else if (failed) {
      chipText = 'Falhou';
      tone = Tone.err;
    } else if (o.status == 'in_flight') {
      chipText = 'Enviando';
      tone = Tone.info;
    } else if (o.hasToken) {
      chipText = 'Selo pronto';
      tone = Tone.info;
    } else {
      chipText = 'Sem selo';
      tone = Tone.warn;
    }

    final subtitle = [
      if (deadline != null) deadline!.title else 'Tarefa #${o.assignId}',
      if (!delivered && due != null) 'vence ${fmtShortDate(due)}',
    ].join(' · ');

    final states = outboxSteps(o.status, o.hasToken);
    final stepLabel = outboxStepLabel(states);
    final highlight = !delivered && !failed && o.hasToken;

    final header = Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(o.filename,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(fontSize: 15, fontWeight: FontWeight.w700, color: p.ink)),
              const SizedBox(height: 2),
              Text(subtitle, maxLines: 2, overflow: TextOverflow.ellipsis, style: TextStyle(fontSize: 12, color: p.muted)),
            ],
          ),
        ),
        const SizedBox(width: 8),
        StatusChip(chipText, tone: tone),
      ],
    );

    final Widget body;
    if (delivered) {
      body = header;
    } else {
      body = Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          header,
          const SizedBox(height: 12),
          Semantics(
            label: 'Etapa atual: $stepLabel',
            excludeSemantics: true,
            child: Column(
              children: [
                Row(
                  children: [
                    for (var i = 0; i < 4; i++) ...[
                      if (i > 0) const SizedBox(width: 4),
                      Expanded(
                        child: Container(
                          height: 4,
                          decoration: BoxDecoration(
                            borderRadius: BorderRadius.circular(2),
                            color: switch (states[i]) {
                              StageState.done => green,
                              StageState.active => Theme.of(context).colorScheme.primary,
                              StageState.error => p.errFg,
                              StageState.pending => p.fieldBorder,
                            },
                          ),
                        ),
                      ),
                    ],
                  ],
                ),
                const SizedBox(height: 6),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    for (final l in outboxStepNames) Text(l, style: TextStyle(fontSize: 11, color: p.muted)),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 12),
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(color: p.bg, borderRadius: BorderRadius.circular(10)),
            child: Text.rich(
              TextSpan(
                style: TextStyle(fontSize: 12, height: 1.5, color: p.mutFg),
                children: [
                  const TextSpan(text: 'SHA-256 '),
                  TextSpan(text: shortHash(o.hash), style: const TextStyle(fontFamily: 'monospace', fontSize: 11)),
                  if (o.retries > 0) TextSpan(text: '\nTentativas: ${o.retries}'),
                ],
              ),
            ),
          ),
          const SizedBox(height: 12),
          OutlinedButton(
            style: OutlinedButton.styleFrom(minimumSize: const Size.fromHeight(44)),
            onPressed: () => showProof(context, store, o.id),
            child: const Text('Ver comprovante'),
          ),
        ],
      );
    }

    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
      child: Material(
        color: p.surface,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: BorderSide(color: highlight ? Theme.of(context).colorScheme.primary : p.border),
        ),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: delivered ? () => showProof(context, store, o.id) : null,
          child: Padding(padding: const EdgeInsets.all(16), child: body),
        ),
      ),
    );
  }
}
