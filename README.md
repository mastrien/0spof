# 0spof (Zero Single Point of Failure)

[![CI](https://github.com/mastrien/0spof/actions/workflows/ci.yml/badge.svg)](https://github.com/mastrien/0spof/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/badge/coverage-%3E%3D70%25-brightgreen.svg)](https://github.com/mastrien/0spof)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**0spof** é um ecossistema offline-first projetado para eliminar o **Single Point of Failure (SPOF)** representado pelas plataformas de aprendizagem institucionais (como o Moodle). 

Ele permite que estudantes consultem materiais, prazos e notas localmente e enfileirem submissões de tarefas durante indisponibilidades do servidor, com **comprovação criptográfica de entrega pré-prazo (RFC 3161 Trusted Timestamping)** e reenvio automático via padrão Outbox.

---

## 🏛️ Arquitetura

O projeto é estruturado como um monorepo Cargo em Rust:

```
0spof/
├── crates/
│   ├── spof-core/       # Motor compartilhado (Sessão, Outbox, SQLite, Prova RFC 3161)
│   ├── spof-cli/        # Interface de Linha de Comando (CLI)
│   ├── spof-mcp/        # Servidor MCP para integração com Agentes de IA
│   └── spof-desktop/    # Aplicação Desktop (Tauri v2)
└── mobile/              # Aplicação Mobile (Flutter - sincronização local)
```

---

## 🛡️ Pilares de Engenharia

- **Desenvolvimento Guiado por Testes (TDD):** Novas funcionalidades são acompanhadas de testes unitários e de integração antes da implementação.
- **Quality Gate no CI:** Mínimo de 70% de cobertura de código obrigatória, com proibição de regressão.
- **Privacidade & LGPD:** Os dados e credenciais pertencem ao usuário e permanecem estritamente em armazenamento local criptografado.
- **Prova de Existência Criptográfica:** Uso de carimbo de tempo auditável (RFC 3161 / TSA) para atestar a conclusão de tarefas antes de prazos de entrega quando o servidor estiver fora do ar.
