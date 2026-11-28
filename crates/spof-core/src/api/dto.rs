use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemStatusDto {
    pub courses_count: usize,
    pub active_courses_count: usize,
    pub muted_courses_count: usize,
    pub deadlines_count: usize,
    pub outbox_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CourseDto {
    pub id: i64,
    pub fullname: String,
    pub shortname: String,
    pub sections_count: usize,
    pub activities_count: usize,
    pub is_muted: bool,
    /// Atividades com prazo futuro ainda não entregues.
    #[serde(default)]
    pub pending_deadlines: usize,
    /// Prazo pendente mais próximo (RFC 3339).
    #[serde(default)]
    pub next_deadline: Option<String>,
    #[serde(default)]
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActivityDto {
    pub id: i64,
    pub section_id: i64,
    pub title: String,
    pub activity_type: String,
    pub url: Option<String>,
    pub description: Option<String>,
    pub local_file_path: Option<String>,
    pub content_mime: Option<String>,
    pub due_date: Option<String>,
    pub is_submitted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SectionDto {
    pub id: i64,
    pub course_id: i64,
    pub name: String,
    pub summary: Option<String>,
    pub activities: Vec<ActivityDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutboxItemDto {
    pub id: i64,
    pub assign_id: i64,
    pub filename: String,
    pub file_hash: String,
    pub status: String,
    pub retry_count: u32,
    pub has_rfc3161_token: bool,
    /// Momento do registro local do item.
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub last_attempt_at: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    /// Horário emitido pela TSA (`genTime`), quando o token é legível.
    #[serde(default)]
    pub timestamped_at: Option<String>,
    /// Prazo da tarefa no Moodle (RFC 3339), se sincronizado.
    #[serde(default)]
    pub due_date: Option<String>,
    /// Folga em segundos entre o selo e o prazo (negativa = selado após o prazo).
    #[serde(default)]
    pub margin_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnqueueResultDto {
    pub outbox_id: i64,
    pub file_hash: String,
    pub has_tsa_token: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofDto {
    pub assign_id: i64,
    pub filename: String,
    pub file_hash: String,
    pub formatted_report: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionInfoDto {
    pub is_authenticated: bool,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncSummaryDto {
    pub courses_synced: usize,
    pub sections_synced: usize,
    pub activities_synced: usize,
    pub resources_cached: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpcomingDeadlineDto {
    pub id: i64,
    pub section_id: i64,
    pub title: String,
    pub activity_type: String,
    pub url: Option<String>,
    pub due_date: Option<String>,
    pub is_submitted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportSummaryDto {
    pub courses_imported: usize,
    pub sections_imported: usize,
    pub activities_imported: usize,
    pub outbox_items_imported: usize,
    #[serde(default)]
    pub files_imported: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportItemPreviewDto {
    pub id: i64,
    pub section_id: i64,
    pub title: String,
    pub activity_type: String,
    pub is_material: bool,
    pub selected_by_default: bool,
    pub is_pdf: bool,
    pub has_local_file: bool,
    pub file_name: Option<String>,
    pub file_size_bytes: Option<u64>,
    pub content_mime: Option<String>,
    pub url: Option<String>,
    pub due_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportSectionPreviewDto {
    pub id: i64,
    pub name: String,
    pub summary: Option<String>,
    pub items: Vec<ExportItemPreviewDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CourseExportPreviewDto {
    pub course_id: i64,
    pub fullname: String,
    pub shortname: String,
    pub suggested_filename: String,
    pub sections: Vec<ExportSectionPreviewDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateInfoDto {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_url: String,
    pub release_notes: Option<String>,
}

/// Uma novidade detectada na última sincronização.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncChangeDto {
    /// `new_resource`, `new_activity`, `deadline_changed` ou `file_cached`.
    pub kind: String,
    pub title: String,
    pub course_short: String,
    /// Detalhe legível (ex.: prazo antigo → novo).
    #[serde(default)]
    pub detail: Option<String>,
}

/// Resumo da última sincronização feita neste aparelho.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncReportDto {
    pub synced_at: String,
    pub courses_synced: usize,
    pub sections_synced: usize,
    pub activities_synced: usize,
    pub resources_cached: usize,
    /// Total de novidades; `changes` traz só as primeiras.
    pub changes_total: usize,
    pub changes: Vec<SyncChangeDto>,
    /// Primeira sincronização deste banco: não há "antes" para comparar, então não há novidades.
    #[serde(default)]
    pub first_sync: bool,
    /// Não foi possível ler o banco para comparar; `changes` não é confiável (vazio).
    #[serde(default)]
    pub comparison_failed: bool,
}

/// Espaço usado localmente.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageInfoDto {
    pub database_bytes: u64,
    pub files_bytes: u64,
    pub files_count: usize,
    pub total_bytes: u64,
}

/// Resultado da busca global local.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchHitDto {
    /// `course`, `activity` (tarefa, questionário, fórum…) ou `resource` (material).
    pub kind: String,
    pub id: i64,
    pub title: String,
    /// Sigla da disciplina e seção, para contexto.
    pub subtitle: String,
    pub course_id: i64,
    pub course_short: String,
    #[serde(default)]
    pub activity_type: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

/// Dados básicos de um arquivo escolhido para entrega.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileInfoDto {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
}
