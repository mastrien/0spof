use serde_json::{json, Value};
use spof_core::outbox::OutboxEngine;
use spof_core::storage::Database;
use std::path::Path;

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

pub async fn execute_tool(db: &Database, name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "get_deadlines" => {
            let deadlines = db.get_upcoming_deadlines().map_err(|e| e.to_string())?;
            let formatted = json!(deadlines
                .into_iter()
                .map(|d| {
                    json!({
                        "id": d.id,
                        "title": d.title,
                        "activity_type": d.activity_type.as_str(),
                        "due_date": d.due_date.map(|t| t.to_rfc3339()),
                        "url": d.url,
                    })
                })
                .collect::<Vec<_>>());

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&formatted).unwrap()
                }]
            }))
        }

        "get_courses" => {
            let courses = db.get_courses().map_err(|e| e.to_string())?;
            let formatted = json!(courses
                .into_iter()
                .map(|c| {
                    json!({
                        "id": c.id,
                        "fullname": c.fullname,
                        "shortname": c.shortname,
                    })
                })
                .collect::<Vec<_>>());

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&formatted).unwrap()
                }]
            }))
        }

        "get_course_sections" => {
            let course_id = args
                .get("course_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| "Parâmetro 'course_id' ausente ou inválido".to_string())?;

            let sections = db
                .get_sections_by_course(course_id)
                .map_err(|e| e.to_string())?;
            let mut result = Vec::new();

            for s in sections {
                let acts = db
                    .get_activities_by_section(s.id)
                    .map_err(|e| e.to_string())?;
                result.push(json!({
                    "section_id": s.id,
                    "name": s.name,
                    "activities": acts.into_iter().map(|a| json!({
                        "id": a.id,
                        "title": a.title,
                        "type": a.activity_type.as_str(),
                        "url": a.url,
                    })).collect::<Vec<_>>()
                }));
            }

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&result).unwrap()
                }]
            }))
        }

        "queue_submission" => {
            let assign_id = args
                .get("assign_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| "Parâmetro 'assign_id' obrigatório".to_string())?;

            let file_path = args
                .get("file_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Parâmetro 'file_path' obrigatório".to_string())?;

            let fetch_tsa = args
                .get("fetch_tsa")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let engine = OutboxEngine::new(db.clone());
            let item = engine
                .enqueue_file(assign_id, Path::new(file_path), fetch_tsa)
                .await
                .map_err(|e| e.to_string())?;

            let res_text = format!(
                "Tarefa #{} enfileirada com sucesso!\nID Outbox: {}\nArquivo: {}\nSHA-256: {}\nCarimbo TSA: {}",
                assign_id,
                item.id.unwrap_or(0),
                item.filename,
                item.file_hash,
                if item.rfc3161_token.is_some() { "Presente" } else { "Ausente" }
            );

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": res_text
                }]
            }))
        }

        "get_submission_proof" => {
            let outbox_id = args
                .get("outbox_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| "Parâmetro 'outbox_id' obrigatório".to_string())?;

            let engine = OutboxEngine::new(db.clone());
            let proof = engine
                .get_submission_proof(outbox_id)
                .map_err(|e| e.to_string())?;

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": proof.formatted_report()
                }]
            }))
        }

        "get_outbox_status" => {
            let items = db.get_pending_outbox_items().map_err(|e| e.to_string())?;
            let formatted = json!(items
                .into_iter()
                .map(|item| {
                    json!({
                        "id": item.id,
                        "assign_id": item.assign_id,
                        "filename": item.filename,
                        "status": item.status.as_str(),
                        "retry_count": item.retry_count,
                        "has_tsa": item.rfc3161_token.is_some(),
                    })
                })
                .collect::<Vec<_>>());

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&formatted).unwrap()
                }]
            }))
        }

        other => Err(format!("Ferramenta desconhecida: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spof_core::models::Course;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_moodle_mcp_tools_flow() {
        let db = Database::open_in_memory().unwrap();

        // 1. List tools
        let tools_list = list_tools();
        let tools_arr = tools_list.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools_arr.len(), 6);

        // 2. get_courses (empty)
        let courses_res = execute_tool(&db, "get_courses", &json!({})).await.unwrap();
        let txt = courses_res["content"][0]["text"].as_str().unwrap();
        assert_eq!(txt, "[]");

        // 3. Insert course and test get_courses
        let c = Course {
            id: 10,
            fullname: "Inteligência Artificial".to_string(),
            shortname: "IA".to_string(),
            summary: None,
            last_synced_at: None,
        };
        db.upsert_course(&c).unwrap();

        let courses_res2 = execute_tool(&db, "get_courses", &json!({})).await.unwrap();
        let txt2 = courses_res2["content"][0]["text"].as_str().unwrap();
        assert!(txt2.contains("Inteligência Artificial"));

        // 4. queue_submission and proof
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"codigo do agente mcp").unwrap();
        temp.flush().unwrap();

        let queue_res = execute_tool(
            &db,
            "queue_submission",
            &json!({
                "assign_id": 99,
                "file_path": temp.path().to_str().unwrap(),
                "fetch_tsa": false
            }),
        )
        .await
        .unwrap();

        let queue_txt = queue_res["content"][0]["text"].as_str().unwrap();
        assert!(queue_txt.contains("Tarefa #99 enfileirada com sucesso!"));

        let proof_res = execute_tool(&db, "get_submission_proof", &json!({"outbox_id": 1}))
            .await
            .unwrap();
        let proof_txt = proof_res["content"][0]["text"].as_str().unwrap();
        assert!(proof_txt.contains("COMPROVANTE CRIPTOGRÁFICO"));

        // 5. get_outbox_status
        let outbox_res = execute_tool(&db, "get_outbox_status", &json!({}))
            .await
            .unwrap();
        let outbox_txt = outbox_res["content"][0]["text"].as_str().unwrap();
        assert!(outbox_txt.contains("pending"));
    }

    #[tokio::test]
    async fn test_unknown_tool_error() {
        let db = Database::open_in_memory().unwrap();
        let res = execute_tool(&db, "ferramenta_inexistente", &json!({})).await;
        assert!(res.is_err());
    }
}
