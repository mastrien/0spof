use serde_json::{json, Value};

/// Returns the MCP tools catalog definitions compliant with MCP specification 2024-11-05.
pub fn list_tools() -> Value {
    json!({
        "tools": [
            {
                "name": "get_deadlines",
                "description": "Lista todos os prazos acadêmicos futuros e tarefas não entregues salvas localmente no Moodle.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "get_courses",
                "description": "Lista todas as disciplinas matriculadas e cacheadas no banco SQLite local.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "get_course_sections",
                "description": "Retorna as seções, tópicos e materiais de aula de uma disciplina específica.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "course_id": {
                            "type": "integer",
                            "description": "ID numérico do curso no Moodle"
                        }
                    },
                    "required": ["course_id"]
                }
            },
            {
                "name": "queue_submission",
                "description": "Enfileira um arquivo local para submissão de uma tarefa no Moodle, calculando o hash SHA-256 e opcionalmente selando com carimbo de tempo público RFC 3161.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "assign_id": {
                            "type": "integer",
                            "description": "ID numérico da tarefa no Moodle"
                        },
                        "file_path": {
                            "type": "string",
                            "description": "Caminho absoluto do arquivo no computador"
                        },
                        "fetch_tsa": {
                            "type": "boolean",
                            "description": "Se deve requisitar o carimbo RFC 3161 (TSA) imediatamente"
                        }
                    },
                    "required": ["assign_id", "file_path"]
                }
            },
            {
                "name": "get_submission_proof",
                "description": "Retorna o comprovante/dossiê criptográfico com carimbo auditável de uma tarefa enfileirada para justificar entrega pré-prazo.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "outbox_id": {
                            "type": "integer",
                            "description": "ID do registro na fila outbox"
                        }
                    },
                    "required": ["outbox_id"]
                }
            },
            {
                "name": "get_outbox_status",
                "description": "Monitora as tarefas pendentes na fila de reenvio outbox.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    })
}
