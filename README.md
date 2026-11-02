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

---

## 🤖 Servidor MCP (Model Context Protocol)

O `0spof-mcp` permite que agentes de IA (como Claude Desktop, Gemini CLI ou Cursor) consultem seus dados do Moodle e gerenciem a fila de entregas:

```json
{
  "mcpServers": {
    "0spof": {
      "command": "0spof-mcp",
      "env": {
        "SPOF_DB_PATH": "C:/path/to/0spof.db"
      }
    }
  }
}
```

### Ferramentas disponíveis via MCP:
- `get_deadlines`: Lista prazos acadêmicos futuros e tarefas não entregues.
- `get_courses`: Lista disciplinas matriculadas e cacheadas localmente.
- `get_course_sections`: Consulta tópicos e materiais de aula da disciplina.
- `queue_submission`: Enfileira arquivo para entrega com cálculo de hash SHA-256 e carimbo de tempo RFC 3161 opcional.
- `get_submission_proof`: Gera dossiê/comprovante criptográfico auditável.
- `get_outbox_status`: Monitora tarefas na fila de reenvio automático.
