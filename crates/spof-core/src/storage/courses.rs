use super::Database;
use crate::error::Result;
use crate::models::{Activity, ActivityType, Course, Section};
use chrono::{DateTime, Utc};
use rusqlite::params;

impl Database {
    // --- Courses ---

    pub fn upsert_course(&self, course: &Course) -> Result<()> {
        let connection = self.conn.lock().unwrap();
        connection.execute(
            r#"
            INSERT INTO courses (id, fullname, shortname, summary, last_synced_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET
                fullname = excluded.fullname,
                shortname = excluded.shortname,
                summary = excluded.summary,
                last_synced_at = excluded.last_synced_at;
            "#,
            params![
                course.id,
                course.fullname,
                course.shortname,
                course.summary,
                course
                    .last_synced_at
                    .map(|timestamp| timestamp.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    pub fn get_courses(&self) -> Result<Vec<Course>> {
        let connection = self.conn.lock().unwrap();
        let mut statement = connection.prepare(
            "SELECT id, fullname, shortname, summary, last_synced_at FROM courses ORDER BY fullname ASC",
        )?;
        let rows = statement.query_map([], |row| {
            let last_synced_str: Option<String> = row.get(4)?;
            Ok(Course {
                id: row.get(0)?,
                fullname: row.get(1)?,
                shortname: row.get(2)?,
                summary: row.get(3)?,
                last_synced_at: last_synced_str.and_then(|timestamp_str| {
                    DateTime::parse_from_rfc3339(&timestamp_str)
                        .ok()
                        .map(|datetime| datetime.with_timezone(&Utc))
                }),
            })
        })?;

        let mut course_list = Vec::new();
        for course_row in rows {
            course_list.push(course_row?);
        }
        Ok(course_list)
    }

    pub fn get_course(&self, course_id: i64) -> Result<Option<Course>> {
        let connection = self.conn.lock().unwrap();
        let mut statement = connection.prepare(
            "SELECT id, fullname, shortname, summary, last_synced_at FROM courses WHERE id = ?1",
        )?;
        let mut rows = statement.query_map(params![course_id], |row| {
            let last_synced_str: Option<String> = row.get(4)?;
            Ok(Course {
                id: row.get(0)?,
                fullname: row.get(1)?,
                shortname: row.get(2)?,
                summary: row.get(3)?,
                last_synced_at: last_synced_str.and_then(|timestamp_str| {
                    DateTime::parse_from_rfc3339(&timestamp_str)
                        .ok()
                        .map(|datetime| datetime.with_timezone(&Utc))
                }),
            })
        })?;

        if let Some(course_result) = rows.next() {
            Ok(Some(course_result?))
        } else {
            Ok(None)
        }
    }

    // --- Sections ---

    pub fn upsert_section(&self, section: &Section) -> Result<()> {
        let connection = self.conn.lock().unwrap();
        connection.execute(
            r#"
            INSERT INTO sections (id, course_id, name, summary)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                course_id = excluded.course_id,
                name = excluded.name,
                summary = excluded.summary;
            "#,
            params![section.id, section.course_id, section.name, section.summary],
        )?;
        Ok(())
    }

    pub fn get_sections_by_course(&self, course_id: i64) -> Result<Vec<Section>> {
        let connection = self.conn.lock().unwrap();
        let mut statement = connection.prepare(
            "SELECT id, course_id, name, summary FROM sections WHERE course_id = ?1 ORDER BY id ASC",
        )?;
        let rows = statement.query_map(params![course_id], |row| {
            Ok(Section {
                id: row.get(0)?,
                course_id: row.get(1)?,
                name: row.get(2)?,
                summary: row.get(3)?,
                activities: Vec::new(),
            })
        })?;

        let mut section_list = Vec::new();
        for section_row in rows {
            section_list.push(section_row?);
        }
        Ok(section_list)
    }

    // --- Activities ---

    pub fn upsert_activity(&self, activity: &Activity) -> Result<()> {
        let connection = self.conn.lock().unwrap();
        connection.execute(
            r#"
            INSERT INTO activities (id, section_id, title, activity_type, url, due_date, is_submitted)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET
                section_id = excluded.section_id,
                title = excluded.title,
                activity_type = excluded.activity_type,
                url = excluded.url,
                due_date = excluded.due_date,
                is_submitted = excluded.is_submitted;
            "#,
            params![
                activity.id,
                activity.section_id,
                activity.title,
                activity.activity_type.as_str(),
                activity.url,
                activity.due_date.map(|timestamp| timestamp.to_rfc3339()),
                if activity.is_submitted { 1 } else { 0 },
            ],
        )?;
        Ok(())
    }

    pub fn get_activities_by_section(&self, section_id: i64) -> Result<Vec<Activity>> {
        let connection = self.conn.lock().unwrap();
        let mut statement = connection.prepare(
            "SELECT id, section_id, title, activity_type, url, due_date, is_submitted FROM activities WHERE section_id = ?1 ORDER BY id ASC",
        )?;
        let rows = statement.query_map(params![section_id], |row| {
            let type_str: String = row.get(3)?;
            let due_date_str: Option<String> = row.get(5)?;
            let is_submitted_flag: i64 = row.get(6)?;
            Ok(Activity {
                id: row.get(0)?,
                section_id: row.get(1)?,
                title: row.get(2)?,
                activity_type: ActivityType::from_moodle_modname(&type_str),
                url: row.get(4)?,
                due_date: due_date_str.and_then(|timestamp_str| {
                    DateTime::parse_from_rfc3339(&timestamp_str)
                        .ok()
                        .map(|datetime| datetime.with_timezone(&Utc))
                }),
                is_submitted: is_submitted_flag == 1,
            })
        })?;

        let mut activity_list = Vec::new();
        for activity_row in rows {
            activity_list.push(activity_row?);
        }
        Ok(activity_list)
    }

    pub fn get_upcoming_deadlines(&self) -> Result<Vec<Activity>> {
        let connection = self.conn.lock().unwrap();
        let mut statement = connection.prepare(
            r#"
            SELECT id, section_id, title, activity_type, url, due_date, is_submitted 
            FROM activities 
            WHERE due_date IS NOT NULL AND is_submitted = 0
            ORDER BY due_date ASC
            "#,
        )?;
        let rows = statement.query_map([], |row| {
            let type_str: String = row.get(3)?;
            let due_date_str: Option<String> = row.get(5)?;
            let is_submitted_flag: i64 = row.get(6)?;
            Ok(Activity {
                id: row.get(0)?,
                section_id: row.get(1)?,
                title: row.get(2)?,
                activity_type: ActivityType::from_moodle_modname(&type_str),
                url: row.get(4)?,
                due_date: due_date_str.and_then(|timestamp_str| {
                    DateTime::parse_from_rfc3339(&timestamp_str)
                        .ok()
                        .map(|datetime| datetime.with_timezone(&Utc))
                }),
                is_submitted: is_submitted_flag == 1,
            })
        })?;

        let mut deadline_list = Vec::new();
        for deadline_row in rows {
            deadline_list.push(deadline_row?);
        }
        Ok(deadline_list)
    }
}
