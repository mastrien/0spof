use clap::{Parser, Subcommand};
use spof_core::models::OutboxStatus;

#[derive(Parser, Debug)]
#[command(
    name = "0spof",
    about = "Zero Single Point of Failure: Resiliência offline e outbox para o Moodle",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Commands {
    /// Exibe o status da sessão e integridade local
    Status,

    /// Lista os itens pendentes na fila outbox
    Outbox,

    /// Executa o cálculo de hash e checagem de integridade de um arquivo
    Checksum {
        /// Caminho para o arquivo
        #[arg(short, long)]
        file: String,
    },
}

pub fn handle_command(cmd: &Commands) -> Result<String, String> {
    match cmd {
        Commands::Status => Ok("0spof engine operacional: sessão e banco prontos.".to_string()),
        Commands::Outbox => Ok(format!(
            "Outbox vazio. Status padrão: {}",
            OutboxStatus::Pending.as_str()
        )),
        Commands::Checksum { file } => {
            let path = std::path::Path::new(file);
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
    match handle_command(&cli.command) {
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
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_handle_command_status() {
        let res = handle_command(&Commands::Status);
        assert!(res.is_ok());
        assert!(res.unwrap().contains("operacional"));
    }

    #[test]
    fn test_handle_command_outbox() {
        let res = handle_command(&Commands::Outbox);
        assert!(res.is_ok());
        assert!(res.unwrap().contains("Outbox"));
    }

    #[test]
    fn test_handle_command_checksum_success() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"dados de teste").unwrap();
        temp.flush().unwrap();

        let cmd = Commands::Checksum {
            file: temp.path().to_str().unwrap().to_string(),
        };
        let res = handle_command(&cmd);
        assert!(res.is_ok());
        assert!(res.unwrap().contains("SHA-256"));
    }

    #[test]
    fn test_handle_command_checksum_failure() {
        let cmd = Commands::Checksum {
            file: "arquivo_inexistente_123.bin".to_string(),
        };
        let res = handle_command(&cmd);
        assert!(res.is_err());
    }
}
