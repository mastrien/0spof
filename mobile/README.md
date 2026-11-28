# 0spof: app do celular (Flutter)

Disciplinas, materiais, prazos e fila de entregas no celular, com banco SQLite próprio (funciona offline).

- A lógica é o mesmo `spof-core` do desktop e da CLI, chamado via
  [flutter_rust_bridge](https://cjycode.com/flutter_rust_bridge/) (`rust/src/api/spof.rs`).
  Estruturas compostas viajam como JSON (`lib/models.dart`). Os DTOs e as consultas ficam em
  `spof_core::api`, compartilhados com o desktop.

## Estado

| Fase | Conteúdo | Situação |
|---|---|---|
| 1 | Ponte Rust, app Flutter, telas de leitura (prazos, disciplinas, entregas, ajustes) | feita |
| 2 | Sincronização com o PC na rede de casa (servidor no núcleo/desktop, QR, cliente no celular) | feita |
| 3 | Login próprio no Moodle (WebView) e crawl local | feita |
| 4 | Registro local de arquivos (outbox, hash e token RFC 3161), download de mídia, exportar/importar, atualização automática e APK no release | feita |

## Como funciona

- **Login próprio:** Ajustes → Entrar no Moodle abre o login institucional (SUAP / Moodle) num WebView.
  O cookie `MoodleSession` é HttpOnly, então é lido pelo `CookieManager` nativo do Android
  (`MainActivity.kt`, canal `zerospof/cookies`) e guardado cifrado com `flutter_secure_storage`.
  O app nunca vê a senha.
- **Sincronização com o PC:** Ajustes → Parear com o PC lê o QR code do app desktop. Depois, o
  celular sincroniza ao abrir, ao voltar para primeiro plano, a cada minuto e no puxar para
  atualizar. Fora de casa, a falha é silenciosa. Detalhes no README da raiz.
- **Entregas:** a fila (outbox) é local. O app copia o arquivo para a pasta dele, calcula o SHA-256
  e pede o carimbo RFC 3161. O registro local (hash e token de tempo, assinatura não verificada) abre tocando no item e não
  é prova de entrega. O app não envia nada ao Moodle nem tenta reenviar: a entrega é manual, pelo
  site, e a fila não sincroniza com o PC.
- **Materiais** baixados ficam em `<documentos do app>/resources` e abrem via FileProvider.
- **Atualização:** o app consulta o último release do GitHub, baixa o APK (conferindo o sha256) e
  abre o instalador do sistema, que pede confirmação.

## Desenvolvimento

Requisitos: Flutter, Android SDK + NDK num caminho **sem espaços** (aqui, `C:\Android\Sdk`) e os alvos
Rust do Android (`rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`).

```sh
flutter_rust_bridge_codegen generate   # depois de mudar rust/src/api
flutter run                            # celular conectado
flutter build apk --release
flutter analyze && flutter test
```

Se o build do Android falhar com "Cannot create directory" ou "Invalid entry name", é estado
sujo do Gradle no Windows: `cd android && ./gradlew --stop`, depois `flutter clean`.
