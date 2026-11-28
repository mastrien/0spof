import 'package:flutter/material.dart';

const _weekdaysLower = ['seg', 'ter', 'qua', 'qui', 'sex', 'sáb', 'dom'];
const _weekdaysFull = [
  'Segunda-feira',
  'Terça-feira',
  'Quarta-feira',
  'Quinta-feira',
  'Sexta-feira',
  'Sábado',
  'Domingo',
];
const _months = [
  'janeiro',
  'fevereiro',
  'março',
  'abril',
  'maio',
  'junho',
  'julho',
  'agosto',
  'setembro',
  'outubro',
  'novembro',
  'dezembro',
];

String _two(int n) => n.toString().padLeft(2, '0');

DateTime dayOf(DateTime d) => DateTime(d.year, d.month, d.day);

/// Dias de calendário entre hoje e `d` (negativo se passou).
int daysUntil(DateTime d, {DateTime? now}) {
  final n = now ?? DateTime.now();
  return dayOf(d).difference(dayOf(n)).inDays;
}

String fmtDue(DateTime d) {
  final days = daysUntil(d);
  final hm = '${_two(d.hour)}:${_two(d.minute)}';
  final date = '${_two(d.day)}/${_two(d.month)}';
  final rel = days == 0
      ? 'hoje'
      : days == 1
          ? 'amanhã'
          : days > 1
              ? 'em $days dias'
              : 'passou há ${-days} d';
  return '$date $hm · $rel';
}

/// "Sábado, 28 de novembro".
String fmtToday(DateTime d) => '${_weekdaysFull[d.weekday - 1]}, ${d.day} de ${_months[d.month - 1]}';

/// "SEG" (para o bloco de data).
String weekdayAbbr(DateTime d) => _weekdaysLower[d.weekday - 1].toUpperCase();

/// "seg, 30/11".
String fmtShortDate(DateTime d) => '${_weekdaysLower[d.weekday - 1]}, ${_two(d.day)}/${_two(d.month)}';

/// "seg, 30/11 às 23:59".
String fmtDueLong(DateTime d) => '${fmtShortDate(d)} às ${_two(d.hour)}:${_two(d.minute)}';

/// "2d 08h" / "5h 12min" / "venceu".
String fmtCountdown(DateTime due, {DateTime? now}) {
  final diff = due.difference(now ?? DateTime.now());
  if (diff.isNegative) return 'venceu';
  final days = diff.inDays;
  final hours = diff.inHours % 24;
  if (days > 0) return '${days}d ${_two(hours)}h';
  final mins = diff.inMinutes % 60;
  return '${diff.inHours}h ${_two(mins)}min';
}

/// "agora" / "há 12 min" / "há 3 h" / "há 2 d".
String fmtAgo(DateTime t, {DateTime? now}) {
  final diff = (now ?? DateTime.now()).difference(t);
  if (diff.inMinutes < 1) return 'agora';
  if (diff.inMinutes < 60) return 'há ${diff.inMinutes} min';
  if (diff.inHours < 24) return 'há ${diff.inHours} h';
  return 'há ${diff.inDays} d';
}

/// Texto curto do prazo para chips de disciplina ("Hoje", "Amanhã", "5 dias").
String fmtDaysChip(DateTime due) {
  final days = daysUntil(due);
  if (days < 0) return 'Atrasado';
  if (days == 0) return 'Hoje';
  if (days == 1) return 'Amanhã';
  return '$days dias';
}

/// Hash curto "9f2c41ab…71d42ac".
String shortHash(String h) => h.length <= 16 ? h : '${h.substring(0, 8)}…${h.substring(h.length - 7)}';

IconData typeIcon(String t) => switch (t) {
      'assign' => Icons.assignment_outlined,
      'resource' => Icons.description_outlined,
      'folder' => Icons.folder_outlined,
      'url' => Icons.link,
      'forum' => Icons.forum_outlined,
      'quiz' => Icons.quiz_outlined,
      _ => Icons.extension_outlined,
    };

String typeLabel(String type) => switch (type) {
      'assign' => 'Tarefa',
      'resource' => 'Arquivo',
      'folder' => 'Pasta',
      'url' => 'Link',
      'forum' => 'Fórum',
      'quiz' => 'Questionário',
      'page' => 'Página',
      _ => type,
    };
