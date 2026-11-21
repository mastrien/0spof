//! Sincronização entre aparelhos na rede de casa.
//!
//! Os ids de disciplinas, seções e atividades vêm do Moodle e são iguais em todos os aparelhos,
//! então a sincronização troca *snapshots* e aplica com `upsert` (que já preserva o que é local,
//! como `is_muted`). Caminhos de arquivo e a fila de entregas não viajam neste protocolo.

pub mod client;
pub mod pairing;
#[cfg(feature = "server")]
pub mod server;
pub mod snapshot;

pub use pairing::Pairing;
pub use snapshot::{Snapshot, SnapshotSummary};
