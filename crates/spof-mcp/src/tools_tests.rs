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
    let course = Course {
        id: 10,
        fullname: "Inteligência Artificial".to_string(),
        shortname: "IA".to_string(),
        summary: None,
        last_synced_at: None,
    };
    db.upsert_course(&course).unwrap();

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
