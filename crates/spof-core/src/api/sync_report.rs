//! Resumo da última sincronização ("novidades") e uso de disco local.
//!
//! A comparação é feita no cliente: um retrato dos dados antes e depois do crawl, guardado em
//! `app_meta` (configuração local, não sincronizada entre aparelhos).

use super::dto::{StorageInfoDto, SyncChangeDto, SyncReportDto, SyncSummaryDto};
use crate::storage::Database;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub const LAST_SYNC_REPORT_KEY: &str = "last_sync_report";
/// Quantas novidades detalhadas guardar; o total real vai em `changes_total`.
pub const MAX_LISTED_CHANGES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivitySnap {
    pub title: String,
    pub activity_type: String,
    pub course_short: String,
    pub due_date: Option<DateTime<Utc>>,
    pub has_file: bool,
}

pub type Snapshot = HashMap<i64, ActivitySnap>;

fn is_material_type(t: &str) -> bool {
    matches!(t, "resource" | "page" | "folder" | "url")
}

/// Retrato de todas as atividades locais, indexado pelo id do Moodle.
/// Propaga erros de leitura: um banco ilegível não pode virar "tudo novo" no diff.
pub fn snapshot(database: &Database) -> Result<Snapshot, String> {
    let mut snap = Snapshot::new();
    for row in database.list_activity_index().map_err(|e| e.to_string())? {
        let act = row.activity;
        snap.insert(
            act.id,
            ActivitySnap {
                title: act.title,
                activity_type: act.activity_type.as_str().to_string(),
                course_short: row.course_short,
                due_date: act.due_date,
                has_file: act.local_file_path.is_some_and(|p| !p.is_empty()),
            },
        );
    }
    Ok(snap)
}

fn change(kind: &str, snap: &ActivitySnap, detail: Option<String>) -> SyncChangeDto {
    SyncChangeDto {
        kind: kind.to_string(),
        title: snap.title.clone(),
        course_short: snap.course_short.clone(),
        detail,
    }
}

/// Novidades entre dois retratos: recursos/atividades novos, prazos alterados e arquivos baixados.
/// Devolve a lista completa, ordenada de forma estável (tipo e depois título).
pub fn diff_snapshots(before: &Snapshot, after: &Snapshot) -> Vec<SyncChangeDto> {
    let mut changes = Vec::new();
    let mut ids: Vec<_> = after.keys().copied().collect();
    ids.sort_unstable();
    for id in ids {
        let now = &after[&id];
        match before.get(&id) {
            None => {
                let kind = if is_material_type(&now.activity_type) {
                    "new_resource"
                } else {
                    "new_activity"
                };
                changes.push(change(kind, now, None));
            }
            Some(old) => {
                if old.due_date != now.due_date {
                    let fmt = |d: Option<DateTime<Utc>>| {
                        d.map(|d| d.to_rfc3339())
                            .unwrap_or_else(|| "sem prazo".to_string())
                    };
                    changes.push(change(
                        "deadline_changed",
                        now,
                        Some(format!("{} → {}", fmt(old.due_date), fmt(now.due_date))),
                    ));
                }
                if !old.has_file && now.has_file {
                    changes.push(change("file_cached", now, None));
                }
            }
        }
    }
    let rank = |k: &str| match k {
        "deadline_changed" => 0,
        "new_activity" => 1,
        "new_resource" => 2,
        _ => 3,
    };
    changes.sort_by(|a, b| {
        rank(&a.kind)
            .cmp(&rank(&b.kind))
            .then_with(|| a.title.cmp(&b.title))
    });
    changes
}

/// Monta o relatório e o grava como "última sincronização".
///
/// Critério de `first_sync`: o retrato "antes" está vazio **e** não existe relatório anterior
/// válido em `app_meta` (este aparelho nunca sincronizou este banco). Assim, um banco que já
/// sincronizou antes, mesmo com zero atividades, não é tratado como primeira sync: o que
/// aparecer depois conta como novidade. `wipe_all_data` apaga o relatório, então apagar tudo
/// e sincronizar de novo volta a ser "primeira sincronização".
///
/// Se `before`/`after` faltarem (erro de leitura), grava sem novidades e marca
/// `comparison_failed`, em vez de tratar tudo como novo.
pub fn record_sync_report(
    database: &Database,
    summary: &SyncSummaryDto,
    before: Option<&Snapshot>,
    after: Option<&Snapshot>,
) -> SyncReportDto {
    // só conta como relatório anterior um valor que ainda lê como relatório (JSON corrompido não)
    let had_previous_report = matches!(last_sync_report(database), Ok(Some(_)));
    let (changes, first_sync, comparison_failed) = match (before, after) {
        (Some(b), Some(_)) if b.is_empty() && !had_previous_report => (Vec::new(), true, false),
        (Some(b), Some(a)) => (diff_snapshots(b, a), false, false),
        _ => (Vec::new(), false, true),
    };
    let report = SyncReportDto {
        synced_at: Utc::now().to_rfc3339(),
        courses_synced: summary.courses_synced,
        sections_synced: summary.sections_synced,
        activities_synced: summary.activities_synced,
        resources_cached: summary.resources_cached,
        changes_total: changes.len(),
        changes: changes.into_iter().take(MAX_LISTED_CHANGES).collect(),
        first_sync,
        comparison_failed,
        save_error: None,
    };
    // Falha de gravação não derruba a sincronização. O relatório (com `save_error`) fica em
    // memória no `Database` e é o que `last_sync_report` devolve até a próxima gravação boa.
    let save_error = serde_json::to_string(&report)
        .map_err(|e| e.to_string())
        .and_then(|json| {
            database
                .meta_set(LAST_SYNC_REPORT_KEY, &json)
                .map_err(|e| e.to_string())
        })
        .err();
    let report = SyncReportDto {
        save_error,
        ..report
    };
    database.set_unsaved_sync_report(match report.save_error {
        Some(_) => serde_json::to_string(&report).ok(),
        None => None,
    });
    report
}

/// Relatório da última sincronização feita neste aparelho (`None` se nunca sincronizou).
pub fn last_sync_report(database: &Database) -> Result<Option<SyncReportDto>, String> {
    if let Some(unsaved) = database.unsaved_sync_report() {
        if let Ok(report) = serde_json::from_str(&unsaved) {
            return Ok(Some(report));
        }
    }
    let raw = database
        .meta_get(LAST_SYNC_REPORT_KEY)
        .map_err(|e| e.to_string())?;
    Ok(raw.and_then(|json| serde_json::from_str(&json).ok()))
}

/// Fecha a sincronização: grava o relatório de novidades e devolve o resumo, levando junto
/// qualquer falha de gravação (`report_save_error`) para a interface avisar o usuário.
pub fn finish_sync(
    database: &Database,
    summary: SyncSummaryDto,
    before: Option<&Snapshot>,
    after: Option<&Snapshot>,
) -> SyncSummaryDto {
    let report = record_sync_report(database, &summary, before, after);
    SyncSummaryDto {
        report_save_error: report.save_error,
        ..summary
    }
}

/// Espaço usado: banco SQLite (páginas alocadas) + arquivos baixados referenciados pelas atividades.
pub fn storage_info(database: &Database) -> Result<StorageInfoDto, String> {
    let database_bytes = database.database_size_bytes().map_err(|e| e.to_string())?;
    let mut files_bytes = 0u64;
    let mut files_count = 0usize;
    for row in database.list_activity_index().map_err(|e| e.to_string())? {
        if let Some(path) = row.activity.local_file_path.filter(|p| !p.is_empty()) {
            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.is_file() {
                    files_bytes += meta.len();
                    files_count += 1;
                }
            }
        }
    }
    Ok(StorageInfoDto {
        database_bytes,
        files_bytes,
        files_count,
        total_bytes: database_bytes + files_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Activity, ActivityType, Course, Section};
    use chrono::Duration;

    fn snap(title: &str, ty: &str, due: Option<DateTime<Utc>>, has_file: bool) -> ActivitySnap {
        ActivitySnap {
            title: title.into(),
            activity_type: ty.into(),
            course_short: "ESW".into(),
            due_date: due,
            has_file,
        }
    }

    #[test]
    fn diff_detects_new_changed_and_cached() {
        let due = Utc::now();
        let mut before = Snapshot::new();
        before.insert(1, snap("Relatório", "assign", Some(due), false));
        before.insert(2, snap("Apostila", "resource", None, false));
        before.insert(3, snap("Igual", "page", None, true));
        let mut after = before.clone();
        after.insert(
            1,
            snap("Relatório", "assign", Some(due + Duration::days(1)), false),
        );
        after.insert(2, snap("Apostila", "resource", None, true));
        after.insert(4, snap("Aula 12", "resource", None, false));
        after.insert(5, snap("Fórum", "forum", None, false));

        let changes = diff_snapshots(&before, &after);
        let kinds: Vec<_> = changes.iter().map(|c| c.kind.as_str()).collect();
        assert_eq!(
            kinds,
            vec![
                "deadline_changed",
                "new_activity",
                "new_resource",
                "file_cached"
            ]
        );
        assert!(changes[0].detail.as_ref().unwrap().contains('→'));
        assert_eq!(changes[2].title, "Aula 12");
        assert_eq!(changes[2].course_short, "ESW");
        assert!(diff_snapshots(&after, &after).is_empty());
    }

    #[test]
    fn deadline_removed_is_reported_as_sem_prazo() {
        let mut before = Snapshot::new();
        before.insert(1, snap("T", "assign", Some(Utc::now()), false));
        let mut after = Snapshot::new();
        after.insert(1, snap("T", "assign", None, false));
        let changes = diff_snapshots(&before, &after);
        assert!(changes[0].detail.as_ref().unwrap().ends_with("sem prazo"));
    }

    fn seed(db: &Database, file: Option<String>) {
        db.upsert_course(&Course {
            id: 1,
            fullname: "Engenharia".into(),
            shortname: "ESW".into(),
            summary: None,
            last_synced_at: None,
            is_muted: false,
        })
        .unwrap();
        db.upsert_section(&Section {
            id: 10,
            course_id: 1,
            name: "S".into(),
            summary: None,
            activities: vec![],
        })
        .unwrap();
        db.upsert_activity(&Activity {
            id: 100,
            section_id: 10,
            title: "Livro".into(),
            activity_type: ActivityType::Resource,
            url: None,
            description: None,
            local_file_path: file,
            content_mime: None,
            due_date: None,
            is_submitted: false,
        })
        .unwrap();
    }

    fn summary() -> SyncSummaryDto {
        SyncSummaryDto {
            courses_synced: 2,
            sections_synced: 5,
            activities_synced: 12,
            resources_cached: 3,
            report_save_error: None,
        }
    }

    #[test]
    fn report_is_recorded_truncated_and_read_back() {
        let db = Database::open_in_memory().unwrap();
        assert!(last_sync_report(&db).unwrap().is_none());

        let mut before = Snapshot::new();
        before.insert(1000, snap("Já existia", "resource", None, false));
        let mut after = before.clone();
        for i in 0..12 {
            after.insert(i, snap(&format!("R{i:02}"), "resource", None, false));
        }
        let report = record_sync_report(&db, &summary(), Some(&before), Some(&after));
        assert_eq!(report.changes_total, 12);
        assert_eq!(report.changes.len(), MAX_LISTED_CHANGES);
        assert!(!report.first_sync && !report.comparison_failed);

        let read = last_sync_report(&db).unwrap().unwrap();
        assert_eq!(read, report);
        assert_eq!(read.resources_cached, 3);
    }

    #[test]
    fn first_sync_does_not_inflate_changes() {
        let db = Database::open_in_memory().unwrap();
        let before = Snapshot::new();
        let mut after = Snapshot::new();
        for i in 0..300 {
            after.insert(i, snap(&format!("R{i}"), "resource", None, false));
        }
        let report = record_sync_report(&db, &summary(), Some(&before), Some(&after));
        assert!(report.first_sync);
        assert_eq!(report.changes_total, 0);
        assert!(report.changes.is_empty());
        assert!(last_sync_report(&db).unwrap().unwrap().first_sync);
    }

    #[test]
    fn empty_database_that_already_synced_is_not_a_first_sync() {
        let db = Database::open_in_memory().unwrap();
        // 1ª sync: banco vazio, sem nada para comparar
        let empty = Snapshot::new();
        let first = record_sync_report(&db, &summary(), Some(&empty), Some(&empty));
        assert!(first.first_sync);
        // 2ª sync: continuava com zero atividades, e agora chegam 3
        let mut after = Snapshot::new();
        for i in 0..3 {
            after.insert(i, snap(&format!("N{i}"), "resource", None, false));
        }
        let second = record_sync_report(&db, &summary(), Some(&empty), Some(&after));
        assert!(!second.first_sync, "já havia relatório anterior");
        assert_eq!(second.changes_total, 3);
    }

    #[test]
    fn corrupted_previous_report_does_not_count_as_previous() {
        let db = Database::open_in_memory().unwrap();
        db.meta_set(LAST_SYNC_REPORT_KEY, "{lixo").unwrap();
        let empty = Snapshot::new();
        let mut after = Snapshot::new();
        after.insert(1, snap("X", "resource", None, false));
        let r = record_sync_report(&db, &summary(), Some(&empty), Some(&after));
        assert!(r.first_sync, "valor ilegível não é relatório anterior");
        assert_eq!(r.changes_total, 0);
        // e foi regravado como relatório válido
        assert!(last_sync_report(&db).unwrap().is_some());
    }

    #[test]
    fn save_failure_is_reported_without_failing_the_sync() {
        let db = Database::open_in_memory().unwrap();
        db.conn
            .lock()
            .unwrap()
            .execute("DROP TABLE app_meta", [])
            .unwrap();
        let empty = Snapshot::new();
        let r = record_sync_report(&db, &summary(), Some(&empty), Some(&empty));
        assert!(r.save_error.is_some());

        let ok_db = Database::open_in_memory().unwrap();
        let ok = record_sync_report(&ok_db, &summary(), Some(&empty), Some(&empty));
        assert!(ok.save_error.is_none());
    }

    fn break_meta(db: &Database) {
        db.conn
            .lock()
            .unwrap()
            .execute("DROP TABLE app_meta", [])
            .unwrap();
    }

    #[test]
    fn save_error_reaches_last_report_and_summary_and_clears_after_a_good_save() {
        let db = Database::open_in_memory().unwrap();
        let empty = Snapshot::new();
        break_meta(&db);

        // fluxo de produção: finish_sync -> resumo da sync
        let finished = finish_sync(&db, summary(), Some(&empty), Some(&empty));
        assert!(finished.report_save_error.is_some());
        // e o que o desktop lê depois (get_last_sync_report)
        let read = last_sync_report(&db)
            .unwrap()
            .expect("relatório em memória");
        assert!(read.save_error.is_some());
        assert_eq!(read.save_error, finished.report_save_error);

        // banco recuperado: próxima sync grava e o aviso some
        db.conn
            .lock()
            .unwrap()
            .execute(
                "CREATE TABLE app_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
                [],
            )
            .unwrap();
        let ok = finish_sync(&db, summary(), Some(&empty), Some(&empty));
        assert!(ok.report_save_error.is_none());
        assert!(last_sync_report(&db).unwrap().unwrap().save_error.is_none());
    }

    #[test]
    fn wipe_resets_first_sync_marker() {
        let db = Database::open_in_memory().unwrap();
        let empty = Snapshot::new();
        record_sync_report(&db, &summary(), Some(&empty), Some(&empty));
        assert!(last_sync_report(&db).unwrap().is_some());
        db.wipe_all_data().unwrap();
        assert!(last_sync_report(&db).unwrap().is_none());
        let mut after = Snapshot::new();
        after.insert(1, snap("X", "resource", None, false));
        let r = record_sync_report(&db, &summary(), Some(&empty), Some(&after));
        assert!(r.first_sync);
    }

    #[test]
    fn unreadable_snapshot_marks_report_untrusted_instead_of_all_new() {
        let db = Database::open_in_memory().unwrap();
        let mut after = Snapshot::new();
        after.insert(1, snap("X", "resource", None, false));
        let report = record_sync_report(&db, &summary(), None, Some(&after));
        assert!(report.comparison_failed);
        assert!(!report.first_sync);
        assert_eq!(report.changes_total, 0);
    }

    #[test]
    fn snapshot_propagates_database_errors() {
        let db = Database::open_in_memory().unwrap();
        db.conn
            .lock()
            .unwrap()
            .execute("DROP TABLE activities", [])
            .unwrap();
        assert!(snapshot(&db).is_err());
        assert!(storage_info(&db).is_err());
    }

    #[test]
    fn old_stored_report_without_new_fields_still_parses() {
        let db = Database::open_in_memory().unwrap();
        db.meta_set(
            LAST_SYNC_REPORT_KEY,
            r#"{"synced_at":"2026-01-01T00:00:00Z","courses_synced":1,"sections_synced":1,"activities_synced":1,"resources_cached":0,"changes_total":0,"changes":[]}"#,
        )
        .unwrap();
        let r = last_sync_report(&db).unwrap().unwrap();
        assert!(!r.first_sync && !r.comparison_failed);
    }

    #[test]
    fn corrupted_report_reads_as_none() {
        let db = Database::open_in_memory().unwrap();
        db.meta_set(LAST_SYNC_REPORT_KEY, "{não é json").unwrap();
        assert!(last_sync_report(&db).unwrap().is_none());
    }

    #[test]
    fn snapshot_reflects_database() {
        let db = Database::open_in_memory().unwrap();
        seed(&db, Some("/tmp/x.pdf".into()));
        let s = snapshot(&db).unwrap();
        assert_eq!(s.len(), 1);
        assert!(s[&100].has_file);
        assert_eq!(s[&100].course_short, "ESW");
    }

    #[test]
    fn storage_info_sums_database_and_existing_files() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("livro.pdf");
        std::fs::write(&file, vec![0u8; 2048]).unwrap();
        let db = Database::open_in_memory().unwrap();
        seed(&db, Some(file.to_string_lossy().to_string()));

        let info = storage_info(&db).unwrap();
        assert_eq!(info.files_bytes, 2048);
        assert_eq!(info.files_count, 1);
        assert!(info.database_bytes > 0);
        assert_eq!(info.total_bytes, info.database_bytes + 2048);

        // arquivo inexistente não conta
        let db2 = Database::open_in_memory().unwrap();
        seed(&db2, Some("/caminho/que/nao/existe.pdf".into()));
        let info2 = storage_info(&db2).unwrap();
        assert_eq!(info2.files_bytes, 0);
        assert_eq!(info2.files_count, 0);
    }
}
