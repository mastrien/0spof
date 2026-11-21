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
| 3 | Login próprio no Moodle (WebView) e crawl local | pendente |
| 4 | Entrega de arquivos (outbox/RFC 3161), download de mídia, exportar/importar, atualização automática e APK no release | pendente |

## Desenvolvimento

Requisitos: Flutter, Android SDK + NDK num caminho **sem espaços** (aqui, `C:\Android\Sdk`) e os alvos
Rust do Android (`rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`).

```sh
flutter_rust_bridge_codegen generate   # depois de mudar rust/src/api
flutter run                            # celular conectado
flutter build apk --release
```
