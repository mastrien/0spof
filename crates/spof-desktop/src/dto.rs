use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemStatusDto {
    pub courses_count: usize,
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
}
