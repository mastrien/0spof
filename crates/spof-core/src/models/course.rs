use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActivityType {
    Assign,
    Resource,
    Folder,
    Url,
    Forum,
    Quiz,
    Other(String),
}

impl ActivityType {
    pub fn from_moodle_modname(name: &str) -> Self {
        match name {
            "assign" => ActivityType::Assign,
            "resource" => ActivityType::Resource,
            "folder" => ActivityType::Folder,
            "url" => ActivityType::Url,
            "forum" => ActivityType::Forum,
            "quiz" => ActivityType::Quiz,
            other => ActivityType::Other(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            ActivityType::Assign => "assign",
            ActivityType::Resource => "resource",
            ActivityType::Folder => "folder",
            ActivityType::Url => "url",
            ActivityType::Forum => "forum",
            ActivityType::Quiz => "quiz",
            ActivityType::Other(name) => name.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Activity {
    pub id: i64,
    pub section_id: i64,
    pub title: String,
    pub activity_type: ActivityType,
    pub url: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub local_file_path: Option<String>,
    #[serde(default)]
    pub content_mime: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub is_submitted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Section {
    pub id: i64,
    pub course_id: i64,
    pub name: String,
    pub summary: Option<String>,
    pub activities: Vec<Activity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Course {
    pub id: i64,
    pub fullname: String,
    pub shortname: String,
    pub summary: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_type_mapping() {
        assert_eq!(
            ActivityType::from_moodle_modname("assign"),
            ActivityType::Assign
        );
        assert_eq!(
            ActivityType::from_moodle_modname("quiz"),
            ActivityType::Quiz
        );
        assert_eq!(
            ActivityType::from_moodle_modname("resource"),
            ActivityType::Resource
        );
        assert_eq!(
            ActivityType::from_moodle_modname("folder"),
            ActivityType::Folder
        );
        assert_eq!(ActivityType::from_moodle_modname("url"), ActivityType::Url);
        assert_eq!(
            ActivityType::from_moodle_modname("forum"),
            ActivityType::Forum
        );
        assert_eq!(
            ActivityType::from_moodle_modname("custom_h5p"),
            ActivityType::Other("custom_h5p".to_string())
        );

        assert_eq!(ActivityType::Assign.as_str(), "assign");
        assert_eq!(ActivityType::Other("my_mod".to_string()).as_str(), "my_mod");
    }

    #[test]
    fn test_course_serialization() {
        let course = Course {
            id: 42,
            fullname: "Estruturas de Dados".to_string(),
            shortname: "ED-2026".to_string(),
            summary: Some("Curso avançado".to_string()),
            last_synced_at: None,
        };

        let json = serde_json::to_string(&course).expect("serialization failed");
        let deserialized: Course = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(course, deserialized);
    }
}
