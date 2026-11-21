use super::{pairing::Pairing, snapshot::Snapshot, SnapshotSummary};
use crate::error::{Result, SpofError};
use crate::storage::Database;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const PEER_KEY: &str = "sync.peer";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncResult {
    pub received: SnapshotSummary,
    pub sent: SnapshotSummary,
    pub server_name: String,
}

fn http() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(4))
        .build()
        .map_err(|e| SpofError::Network(e.to_string()))
}

fn net<E: std::fmt::Display>(e: E) -> SpofError {
    SpofError::Network(format!("Falha ao falar com o PC: {e}"))
}

async fn get_snapshot(client: &reqwest::Client, p: &Pairing) -> Result<Snapshot> {
    let resp = client
        .get(format!("{}/sync/snapshot", p.base_url()))
        .bearer_auth(&p.token)
        .send()
        .await
        .map_err(net)?;
    check(&resp)?;
    resp.json().await.map_err(net)
}

fn check(resp: &reqwest::Response) -> Result<()> {
    match resp.status().as_u16() {
        200..=299 => Ok(()),
        401 => Err(SpofError::Authentication("Token recusado: pareie de novo com o QR code".into())),
        s => Err(SpofError::Network(format!("O PC respondeu com erro HTTP {s}"))),
    }
}

impl Database {
    pub fn paired_peer(&self) -> Result<Option<Pairing>> {
        Ok(self.meta_get(PEER_KEY)?.and_then(|j| Pairing::from_json(&j).ok()))
    }

    pub fn unpair(&self) -> Result<()> {
        self.meta_delete(PEER_KEY)
    }
}

/// Confere o servidor (baixa o snapshot) e só então guarda o pareamento.
pub async fn pair(db: &Database, pairing: &Pairing, device: &str) -> Result<SyncResult> {
    let client = http()?;
    get_snapshot(&client, pairing).await?; // valida host e token antes de persistir
    db.meta_set(PEER_KEY, &pairing.to_json())?;
    sync_now(db, device).await
}

/// Recebe o snapshot do PC e envia o do aparelho.
pub async fn sync_now(db: &Database, device: &str) -> Result<SyncResult> {
    let peer = db
        .paired_peer()?
        .ok_or_else(|| SpofError::Validation("Nenhum PC pareado".into()))?;
    let client = http()?;

    let remote = get_snapshot(&client, &peer).await?;
    let received = db.apply_snapshot(&remote)?;

    let local = db.sync_snapshot(device)?;
    let resp = client
        .post(format!("{}/sync/snapshot", peer.base_url()))
        .bearer_auth(&peer.token)
        .json(&local)
        .send()
        .await
        .map_err(net)?;
    check(&resp)?;
    let sent = SnapshotSummary {
        courses: local.courses.len(),
        sections: local.sections.len(),
        activities: local.activities.len(),
    };
    Ok(SyncResult { received, sent, server_name: remote.device })
}
