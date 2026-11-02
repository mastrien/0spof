use spof_core::storage::Database;
use spof_mcp::process_jsonrpc_line;
use std::env;
use std::path::PathBuf;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = env::var("SPOF_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("0spof.db"));

    let db = match Database::open(&db_path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!(
                "[0spof-mcp] Erro fatal ao abrir banco SQLite {:?}: {}",
                db_path, e
            );
            std::process::exit(1);
        }
    };

    let stdin = io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();
    let mut stdout = io::stdout();

    while let Ok(Some(line)) = lines.next_line().await {
        if let Some(resp_line) = process_jsonrpc_line(&db, &line).await {
            stdout.write_all(resp_line.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}
