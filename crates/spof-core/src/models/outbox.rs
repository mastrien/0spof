use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OutboxStatus {
    Pending,
    InFlight,
    Delivered,
    Failed,
}

impl OutboxStatus {
    pub fn as_str(&self) -> &str {
        match self {
            OutboxStatus::Pending => "pending",
            OutboxStatus::InFlight => "in_flight",
            OutboxStatus::Delivered => "delivered",
            OutboxStatus::Failed => "failed",
        }
    }
}

impl std::str::FromStr for OutboxStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(OutboxStatus::Pending),
            "in_flight" => Ok(OutboxStatus::InFlight),
            "delivered" => Ok(OutboxStatus::Delivered),
            "failed" => Ok(OutboxStatus::Failed),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubmissionPayload {
    pub assign_id: i64,
    pub filename: String,
    pub file_hash: String,
    pub comments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutboxItem {
    pub id: Option<i64>,
    pub assign_id: i64,
    pub filename: String,
    pub file_hash: String,
    pub local_file_path: String,
    pub status: OutboxStatus,
    pub retry_count: u32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub rfc3161_token: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl OutboxItem {
    pub fn new(assign_id: i64, filename: String, file_hash: String, local_path: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            assign_id,
            filename,
            file_hash,
            local_file_path: local_path,
            status: OutboxStatus::Pending,
            retry_count: 0,
            last_attempt_at: None,
            last_error: None,
            rfc3161_token: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn can_retry(&self, max_retries: u32) -> bool {
        self.status != OutboxStatus::Delivered && self.retry_count < max_retries
    }

    pub fn record_failure(&mut self, error_msg: String) {
        self.retry_count += 1;
        self.status = OutboxStatus::Failed;
        self.last_error = Some(error_msg);
        self.last_attempt_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn record_success(&mut self) {
        self.status = OutboxStatus::Delivered;
        self.last_error = None;
        self.last_attempt_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outbox_item_lifecycle() {
        let mut item = OutboxItem::new(
            101,
            "projeto.zip".to_string(),
            "abc123hash".to_string(),
            "/tmp/projeto.zip".to_string(),
        );

        assert_eq!(item.status, OutboxStatus::Pending);
        assert_eq!(item.retry_count, 0);
        assert!(item.can_retry(3));

        // Fail once
        item.record_failure("HTTP 502 Bad Gateway".to_string());
        assert_eq!(item.status, OutboxStatus::Failed);
        assert_eq!(item.retry_count, 1);
        assert_eq!(item.last_error.as_deref(), Some("HTTP 502 Bad Gateway"));
        assert!(item.can_retry(3));

        // Fail until max retries
        item.record_failure("HTTP 504 Gateway Timeout".to_string());
        item.record_failure("Connection Refused".to_string());
        assert_eq!(item.retry_count, 3);
        assert!(!item.can_retry(3));

        // Mark success
        item.record_success();
        assert_eq!(item.status, OutboxStatus::Delivered);
        assert_eq!(item.last_error, None);
        assert!(!item.can_retry(3));
    }

    #[test]
    fn test_outbox_status_conversions() {
        use std::str::FromStr;
        assert_eq!(OutboxStatus::from_str("pending"), Ok(OutboxStatus::Pending));
        assert_eq!(
            OutboxStatus::from_str("in_flight"),
            Ok(OutboxStatus::InFlight)
        );
        assert_eq!(
            OutboxStatus::from_str("delivered"),
            Ok(OutboxStatus::Delivered)
        );
        assert_eq!(OutboxStatus::from_str("failed"), Ok(OutboxStatus::Failed));
        assert!(OutboxStatus::from_str("unknown").is_err());

        assert_eq!(OutboxStatus::Pending.as_str(), "pending");
    }
}
