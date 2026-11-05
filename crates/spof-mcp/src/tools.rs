pub use crate::catalog::list_tools;
use serde_json::{json, Value};
use spof_core::outbox::OutboxEngine;
use spof_core::storage::Database;
use std::path::Path;

/// Executes a requested MCP tool by name against the local SQLite database.
pub async fn execute_tool(
    database: &Database,
    tool_name: &str,
    arguments: &Value,
) -> Result<Value, String> {
    match tool_name {
        "get_deadlines" => {
            let deadlines = database
                .get_upcoming_deadlines()
                .map_err(|e| e.to_string())?;
            let formatted = json!(deadlines
                .into_iter()
                .map(|deadline| {
                    json!({
                        "id": deadline.id,
                        "title": deadline.title,
                        "activity_type": deadline.activity_type.as_str(),
                        "due_date": deadline.due_date.map(|t| t.to_rfc3339()),
                        "url": deadline.url,
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
            let courses = database.get_courses().map_err(|e| e.to_string())?;
            let formatted = json!(courses
                .into_iter()
                .map(|course| {
                    json!({
                        "id": course.id,
                        "fullname": course.fullname,
                        "shortname": course.shortname,
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
            let course_id = arguments
                .get("course_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| "Parâmetro 'course_id' ausente ou inválido".to_string())?;

            let sections = database
                .get_sections_by_course(course_id)
                .map_err(|e| e.to_string())?;
            let mut result = Vec::new();

            for section in sections {
                result.push(json!({
                    "section_id": section.id,
                    "name": section.name,
                    "summary": section.summary,
                    "activities": section.activities.into_iter().map(|activity| json!({
                        "id": activity.id,
                        "title": activity.title,
                        "type": activity.activity_type.as_str(),
                        "url": activity.url,
                        "description": activity.description,
                        "local_file_path": activity.local_file_path,
                        "content_mime": activity.content_mime,
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
            let assign_id = arguments
                .get("assign_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| "Parâmetro 'assign_id' obrigatório".to_string())?;

            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Parâmetro 'file_path' obrigatório".to_string())?;

            let fetch_tsa = arguments
                .get("fetch_tsa")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let engine = OutboxEngine::new(database.clone());
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
            let outbox_id = arguments
                .get("outbox_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| "Parâmetro 'outbox_id' obrigatório".to_string())?;

            let engine = OutboxEngine::new(database.clone());
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
            let items = database
                .get_pending_outbox_items()
                .map_err(|e| e.to_string())?;
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

#[path = "tools_tests.rs"]
#[cfg(test)]
mod tests;
