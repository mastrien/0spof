use crate::error::Result;
use crate::models::{Activity, Course, Section};
use crate::storage::Database;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub version: u32,
    pub device: String,
    pub generated_at: DateTime<Utc>,
    pub courses: Vec<Course>,
    pub sections: Vec<Section>,
    pub activities: Vec<Activity>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotSummary {
    pub courses: usize,
    pub sections: usize,
    pub activities: usize,
}

impl Database {
    /// Fotografia do conteúdo do Moodle, sem caminhos locais nem fila de entregas.
    pub fn sync_snapshot(&self, device: &str) -> Result<Snapshot> {
        let backup = self.export_data()?;
        let activities = backup
            .activities
            .into_iter()
            .map(|mut a| {
                a.local_file_path = None;
                a
            })
            .collect();
        Ok(Snapshot {
            version: SNAPSHOT_VERSION,
            device: device.to_string(),
            generated_at: Utc::now(),
            courses: backup.courses,
            sections: backup.sections,
            activities,
        })
    }

    pub fn apply_snapshot(&self, snapshot: &Snapshot) -> Result<SnapshotSummary> {
        for c in &snapshot.courses {
            self.upsert_course(c)?;
        }
        for s in &snapshot.sections {
            self.upsert_section(s)?;
        }
        for a in &snapshot.activities {
            let mut a = a.clone();
            a.local_file_path = None; // nunca aceitar caminho de outro aparelho
            self.upsert_activity(&a)?;
        }
        Ok(SnapshotSummary {
            courses: snapshot.courses.len(),
            sections: snapshot.sections.len(),
            activities: snapshot.activities.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ActivityType;

    fn seeded() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.upsert_course(&Course {
            id: 1,
            fullname: "Cálculo".into(),
            shortname: "CALC".into(),
            summary: None,
            last_synced_at: None,
            is_muted: false,
        })
        .unwrap();
        db.upsert_section(&Section { id: 10, course_id: 1, name: "S1".into(), summary: None, activities: vec![] })
            .unwrap();
        db.upsert_activity(&Activity {
            id: 100,
            section_id: 10,
            title: "Lista 1".into(),
            activity_type: ActivityType::Assign,
            url: Some("https://m/1".into()),
            description: None,
            local_file_path: Some("C:/pc/lista.pdf".into()),
            content_mime: None,
            due_date: None,
            is_submitted: false,
        })
        .unwrap();
        db
    }

    #[test]
    fn snapshot_strips_local_paths_and_applies_elsewhere() {
        let a = seeded();
        let snap = a.sync_snapshot("PC").unwrap();
        assert!(snap.activities.iter().all(|x| x.local_file_path.is_none()));

        let b = Database::open_in_memory().unwrap();
        let sum = b.apply_snapshot(&snap).unwrap();
        assert_eq!((sum.courses, sum.sections, sum.activities), (1, 1, 1));
        assert_eq!(b.get_activity(100).unwrap().unwrap().title, "Lista 1");
        assert!(b.get_activity(100).unwrap().unwrap().local_file_path.is_none());
    }

    #[test]
    fn apply_keeps_local_mute_and_is_idempotent() {
        let a = seeded();
        let b = seeded();
        b.set_course_muted(1, true).unwrap();
        let snap = a.sync_snapshot("PC").unwrap();
        b.apply_snapshot(&snap).unwrap();
        b.apply_snapshot(&snap).unwrap();
        assert!(b.get_course(1).unwrap().unwrap().is_muted);
        assert_eq!(b.get_courses().unwrap().len(), 1);
    }

    #[test]
    fn meta_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.meta_get("k").unwrap(), None);
        db.meta_set("k", "1").unwrap();
        db.meta_set("k", "2").unwrap();
        assert_eq!(db.meta_get("k").unwrap().as_deref(), Some("2"));
        db.meta_delete("k").unwrap();
        assert_eq!(db.meta_get("k").unwrap(), None);
    }
}
