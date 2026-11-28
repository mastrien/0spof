import 'package:flutter/material.dart';

import 'theme.dart';

/// Rótulo pequeno em caixa alta acima dos títulos.
class Eyebrow extends StatelessWidget {
  final String text;
  final Color? color;
  const Eyebrow(this.text, {super.key, this.color});

  @override
  Widget build(BuildContext context) => Text(
        text.toUpperCase(),
        style: TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.88,
          color: color ?? Pal.of(context).muted,
        ),
      );
}

/// Cabeçalho de página: eyebrow + título grande + ação opcional.
class ScreenHeader extends StatelessWidget {
  final String eyebrow, title;
  final Widget? trailing;
  const ScreenHeader({super.key, required this.eyebrow, required this.title, this.trailing});

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.fromLTRB(16, 20, 16, 0),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Eyebrow(eyebrow),
                  const SizedBox(height: 2),
                  Semantics(
                    header: true,
                    child: Text(
                      title,
                      style: TextStyle(
                        fontSize: 26,
                        fontWeight: FontWeight.w700,
                        letterSpacing: -0.5,
                        color: Pal.of(context).ink,
                      ),
                    ),
                  ),
                ],
              ),
            ),
            ?trailing,
          ],
        ),
      );
}

/// Botão quadrado com borda (visual de 44 dp) usado nos cabeçalhos. Mantém o tap target
/// padrão do Material (48 dp), então a área de toque real é maior que o desenho.
class HeaderIconButton extends StatelessWidget {
  final String tooltip;
  final IconData icon;
  final VoidCallback? onPressed;
  const HeaderIconButton({super.key, required this.tooltip, required this.icon, required this.onPressed});

  @override
  Widget build(BuildContext context) {
    final p = Pal.of(context);
    return IconButton(
      tooltip: tooltip,
      onPressed: onPressed,
      icon: Icon(icon, size: 20),
      style: IconButton.styleFrom(
        backgroundColor: p.surface,
        foregroundColor: p.ink,
        fixedSize: const Size(44, 44),
        side: BorderSide(color: p.fieldBorder),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      ),
    );
  }
}

/// Pílula de status.
class StatusChip extends StatelessWidget {
  final String text;
  final Tone tone;
  final IconData? icon;
  const StatusChip(this.text, {super.key, this.tone = Tone.mut, this.icon});

  @override
  Widget build(BuildContext context) {
    final p = Pal.of(context);
    final fg = p.fgOf(tone);
    return Container(
      constraints: const BoxConstraints(minHeight: 24),
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 2),
      decoration: BoxDecoration(color: p.bgOf(tone), borderRadius: BorderRadius.circular(999)),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (icon != null) ...[Icon(icon, size: 14, color: fg), const SizedBox(width: 4)],
          Flexible(child: Text(text, style: TextStyle(fontSize: 12, fontWeight: FontWeight.w600, color: fg))),
        ],
      ),
    );
  }
}

/// Superfície arredondada com borda (cartão do design).
class SpofCard extends StatelessWidget {
  final Widget child;
  final EdgeInsetsGeometry? padding;
  final EdgeInsetsGeometry margin;
  final Color? color, borderColor;
  const SpofCard({
    super.key,
    required this.child,
    this.padding,
    this.margin = EdgeInsets.zero,
    this.color,
    this.borderColor,
  });

  @override
  Widget build(BuildContext context) {
    final p = Pal.of(context);
    return Padding(
      padding: margin,
      // Material (e não Container com cor) para que InkWell/ListTile filhos mostrem o ripple.
      child: Material(
        color: color ?? p.surface,
        clipBehavior: Clip.antiAlias,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: BorderSide(color: borderColor ?? p.border),
        ),
        child: padding == null ? child : Padding(padding: padding!, child: child),
      ),
    );
  }
}

/// Linha de lista com altura mínima de 56 dp, usada em grupos (Ajustes, recursos).
class ListRow extends StatelessWidget {
  final Widget? leading;
  final Widget title;
  final Widget? subtitle;
  final Widget? trailing;
  final VoidCallback? onTap;
  final double minHeight;
  final bool topBorder;
  final EdgeInsetsGeometry padding;
  const ListRow({
    super.key,
    this.leading,
    required this.title,
    this.subtitle,
    this.trailing,
    this.onTap,
    this.minHeight = 56,
    this.topBorder = false,
    this.padding = const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
  });

  @override
  Widget build(BuildContext context) {
    final p = Pal.of(context);
    return InkWell(
      onTap: onTap,
      child: Container(
        constraints: BoxConstraints(minHeight: minHeight),
        padding: padding,
        decoration: BoxDecoration(
          border: topBorder ? Border(top: BorderSide(color: p.divider)) : null,
        ),
        child: Row(
          children: [
            if (leading != null) ...[leading!, const SizedBox(width: 12)],
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  title,
                  if (subtitle != null) Padding(padding: const EdgeInsets.only(top: 2), child: subtitle),
                ],
              ),
            ),
            if (trailing != null) ...[const SizedBox(width: 8), trailing!],
          ],
        ),
      ),
    );
  }
}

/// Faixa informativa arredondada (ex.: "Tudo protegido").
class InfoBanner extends StatelessWidget {
  final Tone tone;
  final IconData icon;
  final Widget child;
  final EdgeInsetsGeometry margin;
  const InfoBanner({
    super.key,
    required this.tone,
    required this.icon,
    required this.child,
    this.margin = const EdgeInsets.fromLTRB(16, 14, 16, 0),
  });

  @override
  Widget build(BuildContext context) {
    final p = Pal.of(context);
    final fg = p.fgOf(tone);
    return Padding(
      padding: margin,
      child: Container(
        constraints: const BoxConstraints(minHeight: 44),
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        decoration: BoxDecoration(color: p.bgOf(tone), borderRadius: BorderRadius.circular(12)),
        child: DefaultTextStyle(
          style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: fg),
          child: Row(
            children: [
              Icon(icon, size: 18, color: fg),
              const SizedBox(width: 10),
              Expanded(child: child),
            ],
          ),
        ),
      ),
    );
  }
}

/// Rótulo de grupo acima de listas.
class GroupLabel extends StatelessWidget {
  final String text;
  final EdgeInsetsGeometry padding;
  const GroupLabel(this.text, {super.key, this.padding = const EdgeInsets.fromLTRB(20, 14, 20, 8)});

  @override
  Widget build(BuildContext context) => Padding(
        padding: padding,
        child: Text(text, style: TextStyle(fontSize: 12, fontWeight: FontWeight.w600, color: Pal.of(context).muted)),
      );
}

/// Pílula de filtro (36 dp visuais, área de toque de 44 dp).
class FilterPill extends StatelessWidget {
  final String label;
  final bool selected;
  /// Nulo = pílula desabilitada.
  final VoidCallback? onTap;
  const FilterPill({super.key, required this.label, required this.selected, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final p = Pal.of(context);
    final bg = selected ? p.infoBg : p.surface;
    final enabled = onTap != null;
    final fg = !enabled ? p.muted : (selected ? p.infoFg : p.ink);
    return Semantics(
      button: true,
      enabled: enabled,
      selected: selected,
      label: label,
      excludeSemantics: true,
      onTap: onTap,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(22),
        child: ConstrainedBox(
          constraints: const BoxConstraints(minHeight: 44),
          child: Center(
            child: Container(
              constraints: const BoxConstraints(minHeight: 36),
              padding: const EdgeInsets.symmetric(horizontal: 14),
              alignment: Alignment.center,
              decoration: BoxDecoration(
                color: bg,
                borderRadius: BorderRadius.circular(18),
                border: Border.all(color: selected ? Theme.of(context).colorScheme.primary : p.fieldBorder),
              ),
              child: Text(label, style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: fg)),
            ),
          ),
        ),
      ),
    );
  }
}
