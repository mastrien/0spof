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
