use clap::{Parser, Subcommand};
use spof_core::outbox::OutboxEngine;
use spof_core::storage::Database;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "0spof",
    about = "Zero Single Point of Failure: Resiliência offline e outbox para o Moodle",
    version
)]
pub struct Cli {
    /// Caminho customizado para o arquivo de banco SQLite (padrão: ./0spof.db)
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Commands {
    /// Exibe o status da sessão, integridade local e contagem de itens
    Status,

    /// Lista os cursos cacheados localmente
    Courses,

    /// Lista os itens pendentes na fila outbox
    Outbox,

    /// Enfileira uma entrega de tarefa com hash SHA-256 e selagem RFC 3161
    Queue {
        /// ID numérico da tarefa no Moodle
        #[arg(short, long)]
        assign: i64,

        /// Caminho para o arquivo a ser submetido
        #[arg(short, long)]
        file: String,

        /// Se deve requisitar carimbo de tempo da autoridade pública (RFC 3161 TSA)
        #[arg(long, default_value_t = false)]
        tsa: bool,
    },

    /// Emite o dossiê/comprovante criptográfico de uma entrega na fila
    Proof {
        /// ID do item na outbox
        #[arg(short, long)]
        id: i64,
    },

    /// Executa o cálculo de hash e checagem de integridade de um arquivo
    Checksum {
        /// Caminho para o arquivo
        #[arg(short, long)]
        file: String,
    },
}

pub async fn run_command(db: &Database, cmd: &Commands) -> Result<String, String> {
    match cmd {
        Commands::Status => {
            let courses = db.get_courses().map_err(|e| e.to_string())?;
            let pending = db.get_pending_outbox_items().map_err(|e| e.to_string())?;
            let deadlines = db.get_upcoming_deadlines().map_err(|e| e.to_string())?;

            Ok(format!(
                "0spof engine operacional.\nCursos salvos: {}\nPrazos pendentes: {}\nItens na Outbox: {}",
                courses.len(),
                deadlines.len(),
                pending.len()
            ))
        }

        Commands::Courses => {
            let courses = db.get_courses().map_err(|e| e.to_string())?;
            if courses.is_empty() {
                return Ok("Nenhum curso cacheado localmente ainda.".to_string());
            }

            let mut output = String::from("Cursos cacheados:\n");
            for c in courses {
                output.push_str(&format!("- [{}] {}\n", c.shortname, c.fullname));
            }
            Ok(output)
        }

        Commands::Outbox => {
            let items = db.get_pending_outbox_items().map_err(|e| e.to_string())?;
            if items.is_empty() {
                return Ok("Fila Outbox vazia. Nenhuma entrega pendente.".to_string());
            }

            let mut output = format!("Fila Outbox ({} pendentes):\n", items.len());
            for item in items {
                output.push_str(&format!(
                    "- [ID {}] Tarefa {}: {} (Status: {}, Tentativas: {})\n",
                    item.id.unwrap_or(0),
                    item.assign_id,
                    item.filename,
                    item.status.as_str(),
                    item.retry_count
                ));
            }
            Ok(output)
        }

        Commands::Queue { assign, file, tsa } => {
            let path = Path::new(file);
            let engine = OutboxEngine::new(db.clone());
            let item = engine
                .enqueue_file(*assign, path, *tsa)
                .await
                .map_err(|e| e.to_string())?;

            Ok(format!(
                "Tarefa #{} enfileirada com sucesso!\nID Outbox: {}\nArquivo: {}\nSHA-256: {}\nCarimbo TSA: {}",
                assign,
                item.id.unwrap_or(0),
                item.filename,
                item.file_hash,
                if item.rfc3161_token.is_some() { "Selado" } else { "Local" }
            ))
        }

        Commands::Proof { id } => {
            let engine = OutboxEngine::new(db.clone());
            let proof = engine
                .get_submission_proof(*id)
                .map_err(|e| e.to_string())?;

            Ok(proof.formatted_report())
        }

        Commands::Checksum { file } => {
            let path = Path::new(file);
            match spof_core::crypto::hash_file(path) {
                Ok(hash) => Ok(format!("SHA-256 ({}): {}", file, hash)),
                Err(e) => Err(format!(
                    "Erro ao calcular hash do arquivo '{}': {}",
                    file, e
                )),
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(|| PathBuf::from("0spof.db"));
    let db = match Database::open(&db_path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Erro ao inicializar banco de dados {:?}: {}", db_path, e);
            std::process::exit(1);
        }
    };

    match run_command(&db, &cli.command).await {
        Ok(msg) => println!("{}", msg),
        Err(err) => {
            eprintln!("Erro: {}", err);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spof_core::models::Course;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_run_command_status_and_courses() {
        let db = Database::open_in_memory().unwrap();

        let status_res = run_command(&db, &Commands::Status).await;
        assert!(status_res.is_ok());
        assert!(status_res.unwrap().contains("Cursos salvos: 0"));

        let empty_courses = run_command(&db, &Commands::Courses).await;
        assert!(empty_courses.unwrap().contains("Nenhum curso"));

        // Insert dummy course
        let c = Course {
            id: 1,
            fullname: "Arquitetura de Computadores".to_string(),
            shortname: "AC".to_string(),
            summary: None,
            last_synced_at: None,
        };
        db.upsert_course(&c).unwrap();

        let courses_res = run_command(&db, &Commands::Courses).await;
        assert!(courses_res.unwrap().contains("Arquitetura de Computadores"));
    }

    #[tokio::test]
    async fn test_run_command_queue_and_proof() {
        let db = Database::open_in_memory().unwrap();

        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"relatorio de redes").unwrap();
        temp.flush().unwrap();

        let queue_cmd = Commands::Queue {
            assign: 88,
            file: temp.path().to_str().unwrap().to_string(),
            tsa: false,
        };

        let queue_res = run_command(&db, &queue_cmd).await;
        assert!(queue_res.is_ok());
        assert!(queue_res.unwrap().contains("enfileirada com sucesso"));

        // Check outbox
        let outbox_res = run_command(&db, &Commands::Outbox).await;
        assert!(outbox_res.unwrap().contains("ID 1"));

        // Check proof
        let proof_res = run_command(&db, &Commands::Proof { id: 1 }).await;
        assert!(proof_res.is_ok());
        assert!(proof_res.unwrap().contains("COMPROVANTE CRIPTOGRÁFICO"));
    }

    #[tokio::test]
    async fn test_run_command_checksum() {
        let db = Database::open_in_memory().unwrap();
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"dados do teste").unwrap();
        temp.flush().unwrap();

        let check_cmd = Commands::Checksum {
            file: temp.path().to_str().unwrap().to_string(),
        };
        let res = run_command(&db, &check_cmd).await;
        assert!(res.is_ok());
        assert!(res.unwrap().contains("SHA-256"));
    }
}
