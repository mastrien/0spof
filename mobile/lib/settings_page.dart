import 'package:flutter/material.dart';

import 'data_actions.dart';
import 'format.dart';
import 'login_page.dart';
import 'pair_page.dart';
import 'screens.dart';
import 'store.dart';
import 'theme.dart';
import 'widgets.dart';

class SettingsPage extends StatelessWidget {
  final Store store;
  const SettingsPage({super.key, required this.store});

  Widget _chevron(BuildContext context) => Icon(Icons.chevron_right, color: Pal.of(context).muted);

  Widget _title(BuildContext context, String text, {Color? color}) =>
      Text(text, style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: color ?? Pal.of(context).ink));

  Widget _sub(BuildContext context, String text) =>
      Text(text, style: TextStyle(fontSize: 12, color: Pal.of(context).muted));

  Widget _pcRow(BuildContext context) {
    final peer = store.peer;
    final String sub;
    if (peer == null) {
      sub = 'Leia o QR code do app desktop (Ajustes) na rede de casa.';
    } else if (store.pcSyncError != null) {
      sub = store.pcSyncError!;
    } else if (store.pcSyncing) {
      sub = 'Sincronizando…';
    } else if (store.pcLastSync == null) {
      sub = '${peer.host}:${peer.port}';
    } else {
      sub = 'Sincroniza na rede de casa · último sync ${fmtAgo(store.pcLastSync!)}';
    }
    return ListRow(
      title: _title(context, peer == null ? 'Parear com o PC' : 'Pareado com ${peer.name.isEmpty ? peer.host : peer.name}'),
      subtitle: _sub(context, sub),
      trailing: peer == null
          ? _chevron(context)
          : PopupMenuButton<String>(
              tooltip: 'Opções do pareamento',
              onSelected: (v) => v == 'sync' ? store.syncPc() : store.unpairPc(),
              itemBuilder: (_) => const [
                PopupMenuItem(value: 'sync', child: Text('Sincronizar agora')),
                PopupMenuItem(value: 'unpair', child: Text('Desparear')),
              ],
            ),
      onTap: peer == null ? () => Navigator.push(context, MaterialPageRoute(builder: (_) => PairPage(store: store))) : null,
    );
  }

  @override
  Widget build(BuildContext context) {
    final p = Pal.of(context);
    final s = store.status;
    final up = store.update;
    final authed = store.session.authenticated;
    final host = store.session.baseUrl == null ? null : Uri.tryParse(store.session.baseUrl!)?.host;

    final children = <Widget>[
      const ScreenHeader(eyebrow: 'Conta e dados', title: 'Ajustes'),
      const SizedBox(height: 14),
    ];

    if (up != null) {
      children.add(
        SpofCard(
          margin: const EdgeInsets.fromLTRB(16, 0, 16, 14),
          color: p.infoBg,
          borderColor: p.infoBg,
          child: ListRow(
            minHeight: 64,
            leading: Icon(Icons.system_update, color: p.infoFg),
            title: Text('Versão ${up.version} disponível',
                style: TextStyle(fontSize: 15, fontWeight: FontWeight.w700, color: p.infoFg)),
            subtitle: Text(
              store.updateError ??
                  (store.updateProgress != null ? 'Baixando… ${store.updateProgress}%' : 'Toque para baixar e instalar.'),
              style: TextStyle(fontSize: 12, color: p.infoFg),
            ),
            onTap: store.updateProgress == null ? store.installUpdate : null,
          ),
        ),
      );
    }

    if (authed) {
      children.add(
        SpofCard(
          margin: const EdgeInsets.fromLTRB(16, 0, 16, 14),
          color: p.okCardBg,
          borderColor: p.okCardBorder,
          padding: const EdgeInsets.all(14),
          child: Row(
            children: [
              Container(
                width: 40,
                height: 40,
                decoration: const BoxDecoration(color: green, shape: BoxShape.circle),
                child: const Icon(Icons.check, color: Colors.white, size: 20),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Conectado ao Moodle', style: TextStyle(fontSize: 15, fontWeight: FontWeight.w700, color: p.ink)),
                    if (host != null && host.isNotEmpty) ...[
                      const SizedBox(height: 2),
                      Text(host, style: TextStyle(fontSize: 12, color: p.mutFg)),
                    ],
                  ],
                ),
              ),
              TextButton(onPressed: store.logoutMoodle, child: const Text('Sair')),
            ],
          ),
        ),
      );
    }

    children.add(const GroupLabel('Sincronização', padding: EdgeInsets.fromLTRB(20, 0, 20, 8)));
    children.add(dividedCard(context, [
      _pcRow(context),
      if (!authed)
        ListRow(
          title: _title(context, 'Entrar no Moodle'),
          subtitle: _sub(context, 'Login institucional (SUAP). O app nunca vê sua senha.'),
          trailing: _chevron(context),
          onTap: () => Navigator.push(context, MaterialPageRoute(builder: (_) => LoginPage(store: store))),
        ),
      ListRow(
        title: _title(context, 'Sincronizar com o Moodle'),
        subtitle: _sub(context, store.lastError ?? store.lastSyncMessage ?? (authed ? 'Atualiza disciplinas e prazos' : 'Requer sessão ativa')),
        trailing: store.syncing
            ? const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2))
            : Icon(Icons.sync, color: authed ? p.muted : p.divider),
        onTap: authed && !store.syncing ? store.syncMoodle : null,
      ),
    ], margin: const EdgeInsets.symmetric(horizontal: 16)));

    children.add(const GroupLabel('Dados neste aparelho', padding: EdgeInsets.fromLTRB(20, 18, 20, 8)));
    children.add(dividedCard(context, [
      if (s != null)
        ListRow(
          minHeight: 48,
          title: _sub(context,
              '${s.courses} disciplinas (${s.mutedCourses} silenciadas) · ${s.deadlines} prazos · ${s.outbox} na fila'),
        ),
      ListRow(
        title: _title(context, 'Exportar disciplina (.zip)'),
        subtitle: _sub(context, 'Escolha materiais e PDFs para compartilhar'),
        trailing: _chevron(context),
        onTap: () => exportCoursePicker(context, store),
      ),
      ListRow(
        title: _title(context, 'Exportar backup'),
        subtitle: _sub(context, 'Cópia completa em JSON'),
        trailing: _chevron(context),
        onTap: () => exportBackup(context, store),
      ),
      ListRow(
        title: _title(context, 'Importar backup ou disciplina'),
        subtitle: _sub(context, 'Arquivos .json ou .zip'),
        trailing: _chevron(context),
        onTap: () => importPackage(context, store),
      ),
    ]));

    children.add(const GroupLabel('Aplicativo', padding: EdgeInsets.fromLTRB(20, 18, 20, 8)));
    children.add(dividedCard(context, [
      ListRow(
        title: _title(context, 'Versão ${store.version}'),
        subtitle: _sub(context, up == null ? 'Você está na versão mais recente' : 'Atualização disponível'),
        trailing: up == null ? const StatusChip('Atualizado', tone: Tone.ok) : const StatusChip('Nova versão', tone: Tone.info),
      ),
    ]));

    children.add(const SizedBox(height: 14));
    children.add(
      SpofCard(
        margin: const EdgeInsets.symmetric(horizontal: 16),
        borderColor: p.errBorder,
        child: ListRow(
          title: _title(context, 'Apagar todos os dados locais', color: p.errFg),
          subtitle: _sub(context, 'Não afeta o Moodle'),
          onTap: () => wipeLocalData(context, store),
        ),
      ),
    );

    return ListView(
      padding: const EdgeInsets.only(bottom: 24),
      children: children,
    );
  }
}
