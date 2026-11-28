use crate::crypto;
use crate::error::{Result, SpofError};
use crate::models::{OutboxItem, OutboxStatus};
use crate::outbox::tsa::{parse_timestamp_gen_time, TsaClient};
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
    /// Horário emitido pela autoridade de carimbo (`genTime`), quando o token existe e é legível.
    #[serde(default)]
    pub timestamped_at: Option<DateTime<Utc>>,
    /// Prazo da tarefa no Moodle, se já sincronizado.
    #[serde(default)]
    pub due_date: Option<DateTime<Utc>>,
}

/// Formata uma duração em segundos como "2 dias e 13 h", "5 h 10 min" ou "42 min".
pub fn format_margin(total_secs: i64) -> String {
    let secs = total_secs.abs();
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let mins = (secs % 3_600) / 60;
    if days > 0 {
        let d = if days == 1 { "dia" } else { "dias" };
        format!("{days} {d} e {hours} h")
    } else if hours > 0 {
        format!("{hours} h {mins} min")
    } else {
        format!("{mins} min")
    }
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
            timestamped_at: item
                .rfc3161_token
                .as_deref()
                .and_then(parse_timestamp_gen_time),
            due_date: None,
        }
    }

    pub fn with_due_date(mut self, due_date: Option<DateTime<Utc>>) -> Self {
        self.due_date = due_date;
        self
    }

    /// Momento que prova a existência do arquivo: o carimbo da TSA ou, sem ele, o registro local.
    pub fn sealed_at(&self) -> DateTime<Utc> {
        self.timestamped_at.unwrap_or(self.queued_at)
    }

    /// Folga em segundos entre o selo e o prazo (negativa se foi selado depois do prazo).
    pub fn margin_seconds(&self) -> Option<i64> {
        self.due_date
            .map(|due| (due - self.sealed_at()).num_seconds())
    }

    /// Formata o registro local (hash e token de tempo) em texto. Não é prova de entrega.
    pub fn formatted_report(&self) -> String {
        format!(
            r#"================================================================================
0spof - REGISTRO LOCAL DE ARQUIVO (hash e token de tempo, sem prova de entrega)
================================================================================
ID da Tarefa:              {}
Nome do Arquivo:           {}
Checksum SHA-256:          {}
Registrado localmente em:  {} (UTC)
Token RFC 3161:            {} (Tamanho: {} bytes)
Horário lido do token:     {}
Prazo da Tarefa:           {}
Folga em relação ao prazo: {}
Status na fila local:      {}
================================================================================
Este documento NÃO comprova entrega ao Moodle. Ele registra o hash do arquivo
neste computador e, quando presente, o token RFC 3161 obtido da autoridade de
carimbo. O horário é apenas lido do token: a ASSINATURA NÃO É VERIFICADA por
este aplicativo. A entrega ao Moodle é feita por você, fora do 0spof; valide o
token com ferramenta própria (ex.: openssl ts -verify) antes de apresentá-lo.
================================================================================"#,
            self.assign_id,
            self.filename,
            self.file_hash,
            self.queued_at.to_rfc3339(),
            if self.has_rfc3161_token {
                "TOKEN PRESENTE (assinatura não verificada)"
            } else {
                "PENDENTE / SEM TOKEN EXTERNO"
            },
            self.rfc3161_token_bytes_len,
            self.timestamped_at
                .map(|t| format!("{} (UTC)", t.to_rfc3339()))
                .unwrap_or_else(|| "não disponível".to_string()),
            self.due_date
                .map(|d| format!("{} (UTC)", d.to_rfc3339()))
                .unwrap_or_else(|| "não sincronizado".to_string()),
            match self.margin_seconds() {
                Some(m) if m >= 0 => format!(
                    "{} antes do prazo (horário não verificado)",
                    format_margin(m)
                ),
                Some(m) => format!(
                    "{} DEPOIS do prazo (horário não verificado)",
                    format_margin(m)
                ),
                None => "indeterminada".to_string(),
            },
            match self.status {
                OutboxStatus::Pending => "registrado localmente (aguardando envio manual)",
                OutboxStatus::InFlight => "marcado como em envio",
                OutboxStatus::Delivered => "marcado como enviado",
                OutboxStatus::Failed => "falha registrada",
            }
        )
    }
}

/// O OutboxEngine registra arquivos localmente (hash SHA-256 e token RFC 3161 opcional). Não envia ao Moodle.
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

        let due_date = self
            .db
            .get_activity(item.assign_id)?
            .and_then(|activity| activity.due_date);
        Ok(SubmissionProof::from_outbox_item(&item).with_due_date(due_date))
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

    #[test]
    fn test_format_margin() {
        assert_eq!(format_margin(2 * 86_400 + 13 * 3_600), "2 dias e 13 h");
        assert_eq!(format_margin(86_400), "1 dia e 0 h");
        assert_eq!(format_margin(5 * 3_600 + 600), "5 h 10 min");
        assert_eq!(format_margin(-(42 * 60)), "42 min");
    }

    fn item_with_token(token: Option<Vec<u8>>) -> OutboxItem {
        let mut item = OutboxItem::new(7, "a.pdf".into(), "ab".repeat(32), "/tmp/a.pdf".into());
        item.rfc3161_token = token;
        item
    }

    #[test]
    fn test_proof_margin_uses_tsa_time_then_falls_back_to_queue_time() {
        let mut token = vec![0x18, 0x0f];
        token.extend_from_slice(b"20261128100000Z");
        let item = item_with_token(Some(token));
        let due: DateTime<Utc> = "2026-11-30T23:59:00Z".parse().unwrap();
        let proof = SubmissionProof::from_outbox_item(&item).with_due_date(Some(due));
        assert_eq!(
            proof.timestamped_at.unwrap().to_rfc3339(),
            "2026-11-28T10:00:00+00:00"
        );
        assert_eq!(
            proof.margin_seconds(),
            Some(2 * 86_400 + 13 * 3_600 + 59 * 60)
        );
        let report = proof.formatted_report();
        assert!(report.contains("2 dias e 13 h antes do prazo"));
        assert!(report.contains("ASSINATURA NÃO É VERIFICADA"));
        assert!(report.contains("TOKEN PRESENTE (assinatura não verificada)"));
        assert!(!report.contains("VÁLIDO"));
        assert!(report.contains("2026-11-28T10:00:00+00:00 (UTC)"));

        // sem token: usa created_at
        let no_token = item_with_token(None);
        let due2 = no_token.created_at + chrono::Duration::hours(3);
        let p2 = SubmissionProof::from_outbox_item(&no_token).with_due_date(Some(due2));
        assert!(p2.timestamped_at.is_none());
        assert_eq!(p2.margin_seconds(), Some(3 * 3_600));
    }

    #[test]
    fn test_report_never_claims_validity_or_delivery() {
        let mut token = vec![0x18, 0x0f];
        token.extend_from_slice(b"20261128100000Z");
        for status in [
            OutboxStatus::Pending,
            OutboxStatus::InFlight,
            OutboxStatus::Delivered,
            OutboxStatus::Failed,
        ] {
            for with_token in [true, false] {
                let mut item = item_with_token(with_token.then(|| token.clone()));
                item.status = status.clone();
                let report = SubmissionProof::from_outbox_item(&item)
                    .with_due_date(Some(item.created_at))
                    .formatted_report();
                let lower = report.to_lowercase();
                for banned in [
                    "atesta",
                    "válido",
                    "valido",
                    "conclusão",
                    "conclusao",
                    "confirmad",
                    "comprova entrega",
                    "assinado pela autoridade",
                    "delivered",
                    "pending",
                ] {
                    // a única menção permitida é a negação explícita
                    let ok_negation =
                        banned == "comprova entrega" && lower.contains("não comprova entrega");
                    assert!(
                        !lower.contains(banned) || ok_negation,
                        "relatório contém '{banned}':\n{report}"
                    );
                }
                assert!(report.contains("NÃO comprova entrega"));
                assert!(report.contains("ASSINATURA NÃO É VERIFICADA"));
            }
        }
    }

    #[test]
    fn test_proof_reports_late_and_unknown_deadline() {
        let item = item_with_token(None);
        let late = SubmissionProof::from_outbox_item(&item)
            .with_due_date(Some(item.created_at - chrono::Duration::hours(2)));
        assert!(late
            .formatted_report()
            .contains("2 h 0 min DEPOIS do prazo"));
        let unknown = SubmissionProof::from_outbox_item(&item);
        assert_eq!(unknown.margin_seconds(), None);
        assert!(unknown.formatted_report().contains("indeterminada"));
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
        assert!(report.contains("0spof - REGISTRO LOCAL DE ARQUIVO"));
        assert!(report.contains("sem prova de entrega"));
        assert!(!report.to_uppercase().contains("COMPROVANTE"));
        assert!(report.contains(&item.file_hash));
    }
}
