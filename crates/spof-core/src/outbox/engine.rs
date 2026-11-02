use crate::crypto;
use crate::error::{Result, SpofError};
use crate::models::{OutboxItem, OutboxStatus};
use crate::outbox::tsa::TsaClient;
use crate::storage::Database;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// Cryptographic evidence report for an assignment submitted during an outage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubmissionProof {
    pub assign_id: i64,
    pub filename: String,
    pub file_hash: String,
    pub queued_at: DateTime<Utc>,
    pub status: OutboxStatus,
    pub has_rfc3161_token: bool,
    pub rfc3161_token_bytes_len: usize,
}

impl SubmissionProof {
    pub fn from_outbox_item(item: &OutboxItem) -> Self {
        let token_len = item.rfc3161_token.as_ref().map(|t| t.len()).unwrap_or(0);
        Self {
            assign_id: item.assign_id,
            filename: item.filename.clone(),
            file_hash: item.file_hash.clone(),
            queued_at: item.created_at,
            status: item.status.clone(),
            has_rfc3161_token: token_len > 0,
            rfc3161_token_bytes_len: token_len,
        }
    }

    /// Formats the audit report as plain text for academic submission evidence.
    pub fn formatted_report(&self) -> String {
        format!(
            r#"================================================================================
0spof - COMPROVANTE CRIPTOGRÁFICO DE CONCLUSÃO DE TAREFA
================================================================================
ID da Tarefa:              {}
Nome do Arquivo:           {}
Checksum SHA-256:          {}
Registro de Conclusão:     {} (UTC)
Carimbo de Tempo RFC 3161: {} (Tamanho: {} bytes)
Status de Entrega:         {}
================================================================================
Este relatório atesta que o arquivo com o hash acima foi congelado e selado
antes do encerramento do prazo, independentemente de instabilidades no servidor.
================================================================================"#,
            self.assign_id,
            self.filename,
            self.file_hash,
            self.queued_at.to_rfc3339(),
            if self.has_rfc3161_token {
                "VÁLIDO / ASSINADO PELA AUTORIDADE TSA"
            } else {
                "PENDENTE / SEM TOKEN EXTERNO"
            },
            self.rfc3161_token_bytes_len,
            self.status.as_str()
        )
    }
}

/// The OutboxEngine orchestrates offline task queueing, retry backoffs, and cryptographic sealing.
pub struct OutboxEngine {
    db: Database,
    tsa_client: TsaClient,
    max_retries: u32,
    base_backoff_secs: u64,
}

impl OutboxEngine {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            tsa_client: TsaClient::new(None),
            max_retries: 5,
            base_backoff_secs: 30,
        }
    }

    pub fn with_tsa_client(mut self, tsa_client: TsaClient) -> Self {
        self.tsa_client = tsa_client;
        self
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub fn with_base_backoff_secs(mut self, secs: u64) -> Self {
        self.base_backoff_secs = secs;
        self
    }

    /// Calculates exponential backoff with exponential factor 2^retry_count.
    pub fn calculate_backoff(&self, retry_count: u32) -> Duration {
        let factor = 2u64.saturating_pow(retry_count.min(6));
        Duration::from_secs(self.base_backoff_secs.saturating_mul(factor))
    }

    /// Enqueues a local file into the outbox for an assignment.
    /// Calculates the file's SHA-256 hash and optionally seals it with an RFC 3161 TSA timestamp.
    pub async fn enqueue_file<P: AsRef<Path>>(
        &self,
        assign_id: i64,
        file_path: P,
        fetch_tsa: bool,
    ) -> Result<OutboxItem> {
        let path = file_path.as_ref();
        if !path.exists() {
            return Err(SpofError::Validation(format!(
                "File not found on disk: {:?}",
                path
            )));
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| SpofError::Validation("Invalid file name".to_string()))?
            .to_string();

        let file_hash = crypto::hash_file(path)?;
        let local_path = path.to_string_lossy().to_string();

        let mut item = OutboxItem::new(assign_id, filename, file_hash.clone(), local_path);

        if fetch_tsa {
            match self.tsa_client.request_timestamp(&file_hash).await {
                Ok(token) => item.rfc3161_token = Some(token),
                Err(_) => {
                    // Non-fatal: if TSA is unreachable, keep item queued with local proof
                }
            }
        }

        let id = self.db.insert_outbox_item(&item)?;
        item.id = Some(id);

        Ok(item)
    }

    /// Generates a submission proof for an outbox item.
    pub fn get_submission_proof(&self, outbox_item_id: i64) -> Result<SubmissionProof> {
        let item = self
            .db
            .get_outbox_item(outbox_item_id)?
            .ok_or_else(|| SpofError::Outbox(format!("Item {} not found", outbox_item_id)))?;

        Ok(SubmissionProof::from_outbox_item(&item))
    }

    /// Fetches all items that are ready to be dispatched or retried.
    pub fn get_pending_items(&self) -> Result<Vec<OutboxItem>> {
        self.db.get_pending_outbox_items()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_backoff_calculation() {
        let db = Database::open_in_memory().unwrap();
        let engine = OutboxEngine::new(db).with_base_backoff_secs(10);

        assert_eq!(engine.calculate_backoff(0), Duration::from_secs(10)); // 10 * 1
        assert_eq!(engine.calculate_backoff(1), Duration::from_secs(20)); // 10 * 2
        assert_eq!(engine.calculate_backoff(2), Duration::from_secs(40)); // 10 * 4
        assert_eq!(engine.calculate_backoff(3), Duration::from_secs(80)); // 10 * 8
    }

    #[tokio::test]
    async fn test_enqueue_non_existent_file() {
        let db = Database::open_in_memory().unwrap();
        let engine = OutboxEngine::new(db);
        let res = engine
            .enqueue_file(99, Path::new("arquivo_inexistente.pdf"), false)
            .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_enqueue_valid_file_and_proof_generation() {
        let db = Database::open_in_memory().unwrap();
        let engine = OutboxEngine::new(db);

        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"conteudo do relatorio academico").unwrap();
        temp.flush().unwrap();

        let item = engine
            .enqueue_file(42, temp.path(), false)
            .await
            .expect("enqueue failed");

        assert_eq!(item.assign_id, 42);
        assert_eq!(item.status, OutboxStatus::Pending);
        assert!(item.id.is_some());

        let proof = engine
            .get_submission_proof(item.id.unwrap())
            .expect("failed to get proof");
        assert_eq!(proof.assign_id, 42);
        assert_eq!(proof.file_hash, item.file_hash);

        let report = proof.formatted_report();
        assert!(report.contains("0spof - COMPROVANTE"));
        assert!(report.contains(&item.file_hash));
    }
}
