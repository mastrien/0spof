pub mod protocol;
pub mod tools;

use protocol::{JsonRpcRequest, JsonRpcResponse};
use serde_json::json;
use spof_core::storage::Database;

/// Process a single incoming JSON-RPC request and produce an optional response.
/// Notifications return `None`.
pub async fn handle_request(db: &Database, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
    match req.method.as_str() {
        "initialize" => {
            let res = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "0spof-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            });
            Some(JsonRpcResponse::success(req.id, res))
        }
        "notifications/initialized" | "initialized" => {
            // Standard MCP initialized notification
            None
        }
        "ping" => Some(JsonRpcResponse::success(req.id, json!({}))),
        "tools/list" => {
            let tools = tools::list_tools();
            Some(JsonRpcResponse::success(req.id, tools))
        }
        "tools/call" => {
            let params = req.params.unwrap_or(json!({}));
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let default_args = json!({});
            let arguments = params.get("arguments").unwrap_or(&default_args);

            match tools::execute_tool(db, tool_name, arguments).await {
                Ok(val) => Some(JsonRpcResponse::success(req.id, val)),
                Err(err_msg) => Some(JsonRpcResponse::error(req.id, -32603, err_msg)),
            }
        }
        _ => Some(JsonRpcResponse::error(
            req.id,
            -32601,
            format!("Method not found: {}", req.method),
        )),
    }
}

/// Parses and processes a single line of JSON-RPC text (for stdio transport).
pub async fn process_jsonrpc_line(db: &Database, line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
        Ok(r) => r,
        Err(e) => {
            let err_resp = JsonRpcResponse::error(None, -32700, format!("Parse error: {}", e));
            return serde_json::to_string(&err_resp).ok();
        }
    };

    let resp = handle_request(db, req).await?;
    serde_json::to_string(&resp).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_initialize_and_ping() {
        let db = Database::open_in_memory().unwrap();

        // 1. initialize
        let init_line = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let init_res_str = process_jsonrpc_line(&db, init_line).await.unwrap();
        let init_res: serde_json::Value = serde_json::from_str(&init_res_str).unwrap();
        assert_eq!(init_res["jsonrpc"], "2.0");
        assert_eq!(init_res["id"], 1);
        assert_eq!(init_res["result"]["serverInfo"]["name"], "0spof-mcp");

        // 2. initialized notification (should return None)
        let notif_line = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        assert!(process_jsonrpc_line(&db, notif_line).await.is_none());

        // 3. ping
        let ping_line = r#"{"jsonrpc":"2.0","id":"ping-1","method":"ping"}"#;
        let ping_res_str = process_jsonrpc_line(&db, ping_line).await.unwrap();
        let ping_res: serde_json::Value = serde_json::from_str(&ping_res_str).unwrap();
        assert_eq!(ping_res["id"], "ping-1");
    }

    #[tokio::test]
    async fn test_tools_list_and_call() {
        let db = Database::open_in_memory().unwrap();

        // 1. tools/list
        let list_line = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
        let list_res_str = process_jsonrpc_line(&db, list_line).await.unwrap();
        let list_res: serde_json::Value = serde_json::from_str(&list_res_str).unwrap();
        let tools = list_res["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 6);

        // 2. tools/call get_courses
        let call_line = serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "get_courses",
                "arguments": {}
            }
        }))
        .unwrap();

        let call_res_str = process_jsonrpc_line(&db, &call_line).await.unwrap();
        let call_res: serde_json::Value = serde_json::from_str(&call_res_str).unwrap();
        assert_eq!(call_res["id"], 3);
        assert!(call_res["result"]["content"][0]["text"].as_str().is_some());

        // 3. tools/call unknown tool error
        let call_err_line = serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "unknown_tool",
                "arguments": {}
            }
        }))
        .unwrap();

        let call_err_str = process_jsonrpc_line(&db, &call_err_line).await.unwrap();
        let call_err: serde_json::Value = serde_json::from_str(&call_err_str).unwrap();
        assert_eq!(call_err["id"], 4);
        assert_eq!(call_err["error"]["code"], -32603);

        // 4. Unknown method
        let unk_method = r#"{"jsonrpc":"2.0","id":5,"method":"foobar"}"#;
        let unk_res_str = process_jsonrpc_line(&db, unk_method).await.unwrap();
        let unk_res: serde_json::Value = serde_json::from_str(&unk_res_str).unwrap();
        assert_eq!(unk_res["error"]["code"], -32601);

        // 5. Invalid JSON Parse Error
        let invalid_line = r#"{ invalid json"#;
        let invalid_res_str = process_jsonrpc_line(&db, invalid_line).await.unwrap();
        let invalid_res: serde_json::Value = serde_json::from_str(&invalid_res_str).unwrap();
        assert_eq!(invalid_res["error"]["code"], -32700);

        // 6. Empty line
        assert!(process_jsonrpc_line(&db, "   \n").await.is_none());
    }
}
