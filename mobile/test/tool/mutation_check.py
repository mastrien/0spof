#!/usr/bin/env python3
"""Teste de mutação manual do app mobile.

Aplica, um de cada vez, mutações pequenas no código de `lib/` e roda `flutter test`.
Cada mutante precisa ser MORTO (a suíte falha). SOBREVIVEU = buraco de teste.
Uso (de dentro de `mobile/`, com `flutter` no PATH):

    python3 test/tool/mutation_check.py            # todos
    python3 test/tool/mutation_check.py M01 N14    # só os que começam com esses prefixos

O arquivo é sempre restaurado ao final de cada mutante. Sai com código 1 se algum sobreviver
(exceto os equivalentes listados em EQUIVALENT). Fica em test/tool/ (e não no CI) porque leva
vários minutos: cada mutante roda a suíte inteira.
"""
import os
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
env = dict(os.environ, CI='true')
# Mutantes sem efeito observável (equivalentes): não contam como buraco.
EQUIVALENT = {
    'M16',  # `!o.hasToken` redundante: failed já é tratado antes
    'M40',  # minHeight 64 da linha de prazo: o bloco de data (44) + padding (2x10) já dá 64
    'N3',   # `late` só é lido depois de `d.submitted` na cadeia de chips
    'N22',  # sem shrinkWrap o botão tem 48 dp de área de toque (visual 44): continua >= 44
}

S = 'lib/screens.dart'
F = 'lib/format.dart'
T = 'lib/theme.dart'
W = 'lib/widgets.dart'
C = 'lib/courses_pages.dart'
O = 'lib/outbox_page.dart'
X = 'lib/course_export_page.dart'
A = 'lib/activity_actions.dart'
N = 'lib/new_delivery_sheet.dart'

muts=[
('M01 late usa now-1d',S,"} else if (due.isBefore(now)) {","} else if (due.isBefore(now.subtract(const Duration(days: 1)))) {"),
('M02 semana <7',S,"daysUntil(due, now: now) <= 7","daysUntil(due, now: now) < 7"),
('M03 semana <=8',S,"daysUntil(due, now: now) <= 7","daysUntil(due, now: now) <= 8"),
('M04 enviada so com prazo',S,"if (d.submitted) {","if (d.submitted && due != null) {"),
('M05 sem data -> later',S,"g.undated.add(d);","g.later.add(d);"),
('M06 stale >=',S,"difference(pcLastSync) > staleSyncAfter","difference(pcLastSync) >= staleSyncAfter"),
('M07 stale 25h',S,"Duration(hours: 24)","Duration(hours: 25)"),
('M08 stale 23h',S,"Duration(hours: 24)","Duration(hours: 23)"),
('M09 stale sem ago',S,"'Sync desatualizada · ${fmtAgo(pcLastSync, now: now)}'","'Sync desatualizada'"),
('M10 stale tone ok',S,"tone: Tone.warn, icon: warn);\n  }\n  return (text: 'Tudo","tone: Tone.ok, icon: warn);\n  }\n  return (text: 'Tudo"),
('M11 next sem filtro enviada',S,"if (d.submitted || d.due == null || !d.due!.isAfter(now)) continue;","if (d.due == null || !d.due!.isAfter(now)) continue;"),
('M12 next sem preferir assign',S,"if (d.type == 'assign') return d;","if (false) return d;"),
('M13 sort invertido',S,"return a.due!.compareTo(b.due!);","return b.due!.compareTo(a.due!);"),
('M14 ordem grupos',S,"      group('Esta semana', g.week);\n      group('Depois', g.later);","      group('Depois', g.later);\n      group('Esta semana', g.week);"),
('M15 pending conta delivered',S,"final pending = s.outbox.where((o) => o.status != 'delivered').length;\n        return Scaffold","final pending = s.outbox.length;\n        return Scaffold"),
('M16 unprotected sempre',S,"o.status != 'failed' && !o.hasToken","o.status != 'failed'"),
('M17 failed ignora status',S,"open.where((o) => o.status == 'failed').length","open.where((o) => o.status != 'pending').length"),
('M18 erro sync perde prioridade',S,"if (pcSyncError != null) {","if (pcSyncError != null && failed == 0) {"),
('M19 countdown horas sem %24',F,"final hours = diff.inHours % 24;","final hours = diff.inHours;"),
('M20 daysUntil por horas',F,"dayOf(d).difference(dayOf(n)).inDays","d.difference(n).inDays"),
('M21 fmtAgo 24 inclusivo',F,"if (diff.inHours < 24)","if (diff.inHours <= 24)"),
('M22 fmtAgo agora<2',F,"diff.inMinutes < 1","diff.inMinutes < 2"),
('M23 mes marco',F,"'março'","'marco'"),
('M24 weekday troca',F,"['seg', 'ter', 'qua', 'qui', 'sex', 'sáb', 'dom']","['dom', 'seg', 'ter', 'qua', 'qui', 'sex', 'sáb']"),
('M25 shortHash 15',F,"h.length <= 16","h.length <= 15"),
('M26 fmtDaysChip atrasado <=0',F,"if (days < 0) return 'Atrasado';","if (days <= 0) return 'Atrasado';"),
('M27 fmtDue hoje',F,"days == 0\n      ? 'hoje'","days == 0\n      ? 'agora'"),
('M28 ok fg = warnFg',T,"Tone.ok => okFg,","Tone.ok => warnFg,"),
('M29 err bg = infoBg',T,"Tone.err => errBg,","Tone.err => infoBg,"),
('M30 dark hero = claro',T,"hero: Color(0xFF22305A)","hero: Color(0xFF14203A)"),
('M31 dark okCardBg trocado',T,"okCardBg: Color(0xFF12301F)","okCardBg: Color(0xFFFFFFFF)"),
('M32 dark errBorder branco',T,"errBorder: Color(0xFF6B3A33)","errBorder: Color(0xFFFFFFFF)"),
('M33 dark warnCardBg branco',T,"warnCardBg: Color(0xFF2B2210)","warnCardBg: Color(0xFFFFFFFF)"),
('M34 dark divider branco',T,"divider: Color(0xFF222B45)","divider: Color(0xFFFFFFFF)"),
('M35 dark softFill branco',T,"softFill: Color(0xFF131B30)","softFill: Color(0xFFFFFFFF)"),
('M36 dark fieldBorder',T,"fieldBorder: Color(0xFF3A4668)","fieldBorder: Color(0xFF0E1424)"),
('M37 FilterPill sem selected',W,"      selected: selected,\n      label: label,","      label: label,"),
('M38 FilterPill sem button',W,"      button: true,\n      enabled: enabled,","      enabled: enabled,"),
('M39 Row semanticLabel',S,"Icon(typeIcon(d.type), color: p.muted, semanticLabel: typeLabel(d.type))","Icon(typeIcon(d.type), color: p.muted)"),
('M40 Row minHeight 64->40',S,"BoxConstraints(minHeight: 64)","BoxConstraints(minHeight: 40)"),
('M41 data block minWidth 44->30',S,"BoxConstraints(minWidth: 44, minHeight: 44)","BoxConstraints(minWidth: 30, minHeight: 30)"),
('M42 data block tamanho fixo (bug orig)',S,"constraints: const BoxConstraints(minWidth: 44, minHeight: 44),\n              padding: const EdgeInsets.symmetric(horizontal: 2, vertical: 2),","width: 44, height: 44,"),
('M43 SpofCard volta a Container',W,"child: Material(\n        color: color ?? p.surface,\n        clipBehavior: Clip.antiAlias,","child: Material(\n        color: Colors.transparent,\n        clipBehavior: Clip.antiAlias,"),
('M44 StatusChip height fixo',W,"constraints: const BoxConstraints(minHeight: 24),\n      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 2),\n      decoration: BoxDecoration(color: p.bgOf(tone)","height: 24,\n      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 2),\n      decoration: BoxDecoration(color: p.bgOf(tone)"),
('M45 InfoBanner minHeight 44->20',W,"const BoxConstraints(minHeight: 44),\n        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10)","const BoxConstraints(minHeight: 20),\n        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10)"),
('M46 HeaderIconButton 44->32',W,"fixedSize: const Size(44, 44),\n        tapTargetSize: MaterialTapTargetSize.shrinkWrap,\n        side: BorderSide(color: p.fieldBorder),\n        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),\n      ),\n    );\n  }\n}\n\n/// Pílula de status","fixedSize: const Size(32, 32),\n        tapTargetSize: MaterialTapTargetSize.shrinkWrap,\n        side: BorderSide(color: p.fieldBorder),\n        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),\n      ),\n    );\n  }\n}\n\n/// Pílula de status"),
('M47 ListRow minHeight 56->30',W,"this.minHeight = 56","this.minHeight = 30"),
('M48 enqueueFile ignora fetchTsa',A,"fetchTsa: fetchTsa);","fetchTsa: true);"),
('M49 sheet ignora stamp',N,"fetchTsa: _stamp","fetchTsa: true"),
('M50 nav label ativo = ink',T,"s.contains(WidgetState.selected) ? p.infoFg : p.ink,\n        ),\n      ),\n      iconTheme","s.contains(WidgetState.selected) ? p.ink : p.ink,\n        ),\n      ),\n      iconTheme"),
('M51 nav icone ativo muted',T,"s.contains(WidgetState.selected) ? p.infoFg : p.muted","s.contains(WidgetState.selected) ? p.muted : p.muted"),
('M52 filledButton min 48->36',T,"minimumSize: const Size(44, 48)","minimumSize: const Size(44, 36)"),
('M53 textButton 44->30',T,"TextButton.styleFrom(minimumSize: const Size(44, 44))","TextButton.styleFrom(minimumSize: const Size(30, 30))"),
('M54 FAB cor',T,"backgroundColor: action,\n      foregroundColor: onAction,\n      extendedTextStyle","backgroundColor: p.surface,\n      foregroundColor: p.surface,\n      extendedTextStyle"),
('M55 wipe botao cor',  'lib/data_actions.dart',"backgroundColor: Pal.of(d).errFg,\n            foregroundColor: Pal.of(d).surface,","backgroundColor: Pal.of(d).surface,\n            foregroundColor: Pal.of(d).surface,"),
]
muts += [
    ('N1 SpofCard sem Material (ripple escondido)', W,
     "shape: RoundedRectangleBorder(\n          borderRadius: BorderRadius.circular(16),\n          side: BorderSide(color: borderColor ?? p.border),\n        ),\n        child: padding == null ? child : Padding(padding: padding!, child: child),",
     "child: DecoratedBox(decoration: BoxDecoration(color: color ?? p.surface), child: padding == null ? child : Padding(padding: padding!, child: child)),"),
    ('N3 linha atrasada ignora enviada', S, "due.isBefore(now) && !d.submitted;", "due.isBefore(now);"),
    ('N4 rotulo Enviadas', S, "group('Enviadas', g.done);", "group('Concluídas', g.done);"),
    ('N5 rotulo Atrasadas', S, "group('Atrasadas', g.late);", "group('Vencidas', g.late);"),
    ('N7 outboxSteps pending invertido', O, "_ => hasToken ? [d, d, n, n] : [d, n, n, n],", "_ => hasToken ? [d, a, n, n] : [d, d, n, n],"),
    ('N8 outboxSteps failed selo', O, "'failed' => [d, selo, e, n],", "'failed' => [d, d, e, n],"),
    ('N9 outboxSteps inflight selo', O, "'in_flight' => [d, selo, a, n],", "'in_flight' => [d, d, a, n],"),
    ('N10 rotulo Aguardando', O, "return 'Aguardando envio';", "return 'Registrado';"),
    ('N11 contagem regressiva do hero', S, "TextSpan(text: fmtCountdown(due, now: now)", "TextSpan(text: fmtDaysChip(due)"),
    ('N13 chip selo no hero sempre', S, "if (queued.hasToken) const _HeroChip", "if (true) const _HeroChip"),
    ('N14 pickAndEnqueue sem carimbo', A, "await enqueueFile(context, store, assignId, path);", "await enqueueFile(context, store, assignId, path, fetchTsa: false);"),
    ('N15 pendente sem selo volta a "em andamento"', O, "[d, n, n, n],", "[d, a, n, n],"),
    ('N16 rotulo Sem selo', O, "if (s[1] == StageState.pending) return 'Sem selo';", ""),
    ('N17 tile do recurso tamanho fixo', C, "constraints: const BoxConstraints(minWidth: 36, minHeight: 36),", "width: 36, height: 36,"),
    ('N18 linha usa relogio proprio', S, "due.isBefore(now) && !d.submitted;", "due.isBefore(DateTime.now()) && !d.submitted;"),
    ('N19 banner usa relogio proprio', S, "      pcLastSync: store.pcLastSync,\n      now: now,", "      pcLastSync: store.pcLastSync,"),
    ('N20 hero CTA enfileirar -> abrir', S, "label = 'Enfileirar entrega';", "label = 'Abrir';"),
    ('N21 chip Sem arquivo some', S, "? const StatusChip('Sem arquivo', tone: Tone.warn)", "? null"),
    ('N22 tapTarget do botao do cabecalho volta', W, "tapTargetSize: MaterialTapTargetSize.shrinkWrap,", ""),
]

only = sys.argv[1:]
survivors = []
for name, f, old, new in muts:
    if only and not any(name.startswith(o) for o in only):
        continue
    path = os.path.join(ROOT, f)
    with open(path, encoding='utf-8') as fh:
        orig = fh.read()
    if old not in orig:
        print(f'{name}: NAO-APLICOU (atualize o trecho em {f})', flush=True)
        survivors.append(name)
        continue
    with open(path, 'w', encoding='utf-8') as fh:
        fh.write(orig.replace(old, new, 1))
    try:
        r = subprocess.run(['flutter', 'test'], cwd=ROOT, env=env, capture_output=True, text=True, timeout=600)
        out = r.stdout + r.stderr
        if r.returncode == 0:
            status = 'SOBREVIVEU'
        elif 'Error:' in out and 'Some tests failed' not in out:
            status = 'MORTO (erro de compilação: mutante inválido)'
        else:
            status = 'MORTO'
    except Exception as e:  # noqa: BLE001
        status = f'ERRO {e}'
    finally:
        with open(path, 'w', encoding='utf-8') as fh:
            fh.write(orig)
    print(f'{name}: {status}', flush=True)
    if status == 'SOBREVIVEU' and name.split()[0] not in EQUIVALENT:
        survivors.append(name)

print('\nSobreviventes:', survivors or 'nenhum')
sys.exit(1 if survivors else 0)
