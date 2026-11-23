use crate::models::{Activity, ActivityType, Course, Section};
use crate::storage::{
    is_material_activity, is_pdf_activity, suggested_course_export_filename, BackupData,
    CourseExportSelection, Database, ARCHIVE_MANIFEST_NAME,
};
use chrono::Utc;
use std::io::{Cursor, Read, Write};
use tempfile::TempDir;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

fn sample_course(id: i64, fullname: &str, is_muted: bool) -> Course {
    Course {
        id,
        fullname: fullname.to_string(),
        shortname: format!("C-{id}"),
        summary: Some("Ementa".to_string()),
        last_synced_at: Some(Utc::now()),
        is_muted,
    }
}

fn sample_activity(id: i64, section_id: i64, title: &str, activity_type: ActivityType) -> Activity {
    Activity {
        id,
        section_id,
        title: title.to_string(),
        activity_type,
        url: None,
        description: Some("Descrição".to_string()),
        local_file_path: None,
        content_mime: None,
        due_date: Some(Utc::now()),
        is_submitted: false,
    }
}

#[test]
fn classification_distinguishes_materials_activities_and_pdfs() {
    let mut pdf_resource = sample_activity(1, 10, "Apostila 1", ActivityType::Resource);
    pdf_resource.local_file_path = Some("/tmp/1_apostila.pdf".to_string());
    pdf_resource.content_mime = Some("application/pdf".to_string());
    assert!(is_material_activity(&pdf_resource));
    assert!(is_pdf_activity(&pdf_resource));

    let mut url_pdf = sample_activity(2, 10, "Slides", ActivityType::Url);
    url_pdf.url = Some("https://example.org/aula1.PDF?download=1".to_string());
    assert!(is_material_activity(&url_pdf));
    assert!(is_pdf_activity(&url_pdf));

    let title_pdf = sample_activity(3, 10, "Guia.pdf", ActivityType::Resource);
    assert!(is_material_activity(&title_pdf));
    assert!(is_pdf_activity(&title_pdf));

    let mut page_mat = sample_activity(4, 10, "Leitura", ActivityType::Other("page".to_string()));
    page_mat.content_mime = Some("text/html".to_string());
    assert!(is_material_activity(&page_mat));
    assert!(!is_pdf_activity(&page_mat));

    let folder_mat = sample_activity(5, 10, "Pasta", ActivityType::Folder);
    assert!(is_material_activity(&folder_mat));

    let assign_act = sample_activity(6, 10, "Lista 1", ActivityType::Assign);
    assert!(!is_material_activity(&assign_act));

    let quiz_act = sample_activity(7, 10, "Prova", ActivityType::Quiz);
    assert!(!is_material_activity(&quiz_act));

    let forum_act = sample_activity(8, 10, "Avisos", ActivityType::Forum);
    assert!(!is_material_activity(&forum_act));
}

#[test]
fn suggested_filename_strips_code_prefix_and_accents() {
    let course = sample_course(
        86,
        "[20251.3.JCR.TCN.ADS.2017.1I.SUP.18077 (JCRBDD1) 357203] BANCO DE DADOS 1",
        false,
    );
    assert_eq!(
        suggested_course_export_filename(&course, "2026-11-22"),
        "0spof-banco-de-dados-1-2026-11-22.zip"
    );

    let accented = sample_course(99, "Programação & Cálculo", false);
    assert_eq!(
        suggested_course_export_filename(&accented, "2026-11-22"),
        "0spof-programacao-calculo-2026-11-22.zip"
    );
}

#[test]
fn selective_export_bundles_only_selected_items_and_roundtrips_cleanly() {
    let src_dir = TempDir::new().unwrap();
    let pdf1_path = src_dir.path().join("101_Aula_01.pdf");
    let pdf2_path = src_dir.path().join("102_Aula_02.pdf");
    let pdf1_bytes = b"%PDF-1.4\n%conteudo aula 1\n%%EOF".to_vec();
    let pdf2_bytes = b"%PDF-1.4\n%conteudo aula 2 secreto\n%%EOF".to_vec();
    std::fs::write(&pdf1_path, &pdf1_bytes).unwrap();
    std::fs::write(&pdf2_path, &pdf2_bytes).unwrap();

    let db1 = Database::open_in_memory().unwrap();
    db1.upsert_course(&sample_course(42, "Compiladores", true))
        .unwrap();
    db1.upsert_course(&sample_course(99, "Outra Disciplina", false))
        .unwrap();
    db1.upsert_section(&Section {
        id: 420,
        course_id: 42,
        name: "Semana 1: Léxico".to_string(),
        summary: Some("Resumo".to_string()),
        activities: Vec::new(),
    })
    .unwrap();
    db1.upsert_section(&Section {
        id: 421,
        course_id: 42,
        name: "Semana 2: Sintático".to_string(),
        summary: None,
        activities: Vec::new(),
    })
    .unwrap();

    let mut act101 = sample_activity(101, 420, "Slides Aula 01", ActivityType::Resource);
    act101.url = Some("https://moodle.test/mod/resource/view.php?id=101".to_string());
    act101.local_file_path = Some(pdf1_path.to_str().unwrap().to_string());
    act101.content_mime = Some("application/pdf".to_string());
    db1.upsert_activity(&act101).unwrap();

    let mut act102 = sample_activity(102, 421, "Slides Aula 02", ActivityType::Resource);
    act102.url = Some("https://moodle.test/mod/resource/view.php?id=102".to_string());
    act102.local_file_path = Some(pdf2_path.to_str().unwrap().to_string());
    act102.content_mime = Some("application/pdf".to_string());
    db1.upsert_activity(&act102).unwrap();

    let mut act103 = sample_activity(103, 420, "Trabalho 1", ActivityType::Assign);
    act103.url = Some("https://moodle.test/mod/assign/view.php?id=103".to_string());
    act103.is_submitted = true;
    db1.upsert_activity(&act103).unwrap();

    // Seleciona apenas o PDF 101 e a tarefa 103 (deixando o PDF 102 e a seção 421 de fora)
    let zip_bytes = db1
        .export_course_package(&CourseExportSelection {
            course_id: 42,
            activity_ids: vec![101, 103],
            include_section_ids: Vec::new(),
            include_files: true,
        })
        .expect("export zip should succeed");

    // Inspeciona o ZIP gerado
    {
        let mut archive = ZipArchive::new(Cursor::new(&zip_bytes)).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().unwrap().into_owned())
            .collect();
        assert!(names.contains(&ARCHIVE_MANIFEST_NAME.to_string()));
        assert!(names.iter().any(|n| n.ends_with("101_Aula_01.pdf")));
        assert!(!names.iter().any(|n| n.contains("102_Aula_02")));

        let mut manifest_str = String::new();
        archive
            .by_name(ARCHIVE_MANIFEST_NAME)
            .unwrap()
            .read_to_string(&mut manifest_str)
            .unwrap();
        let manifest: BackupData = serde_json::from_str(&manifest_str).unwrap();
        assert_eq!(manifest.version, 2);
        assert_eq!(manifest.courses.len(), 1);
        assert!(!manifest.courses[0].is_muted);
        assert_eq!(manifest.sections.len(), 1);
        assert_eq!(manifest.activities.len(), 2);
        assert!(manifest.outbox_items.is_empty());
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].activity_id, 101);
        // Caminhos locais e estado de entrega do remetente são sanitizados
        assert!(manifest
            .activities
            .iter()
            .all(|a| a.local_file_path.is_none() && !a.is_submitted));
    }

    // Importa em outro dispositivo com outro diretório de cache
    let dst_cache = TempDir::new().unwrap();
    let db2 = Database::open_in_memory().unwrap();
    let summary = db2
        .import_package_bytes(&zip_bytes, dst_cache.path())
        .expect("import zip should succeed");

    assert_eq!(summary.courses_imported, 1);
    assert_eq!(summary.sections_imported, 1);
    assert_eq!(summary.activities_imported, 2);
    assert_eq!(summary.files_imported, 1);

    let act101_dst = db2.get_activity(101).unwrap().unwrap();
    let imported_path = act101_dst
        .local_file_path
        .as_deref()
        .expect("local_file_path must point to extracted file");
    assert!(std::path::Path::new(imported_path).starts_with(dst_cache.path()));
    assert_eq!(std::fs::read(imported_path).unwrap(), pdf1_bytes);

    let act103_dst = db2.get_activity(103).unwrap().unwrap();
    assert!(!act103_dst.is_submitted);
    assert!(db2.get_activity(102).unwrap().is_none());
}

#[test]
fn export_without_files_omits_binary_attachments() {
    let src_dir = TempDir::new().unwrap();
    let pdf_path = src_dir.path().join("101_material.pdf");
    std::fs::write(&pdf_path, b"%PDF-1.4\nhello").unwrap();

    let db = Database::open_in_memory().unwrap();
    db.upsert_course(&sample_course(1, "Redes", false)).unwrap();
    db.upsert_section(&Section {
        id: 10,
        course_id: 1,
        name: "Tópico 1".to_string(),
        summary: None,
        activities: Vec::new(),
    })
    .unwrap();
    let mut act101 = sample_activity(101, 10, "Material", ActivityType::Resource);
    act101.url = Some("https://moodle.test/r/101".to_string());
    act101.local_file_path = Some(pdf_path.to_str().unwrap().to_string());
    act101.content_mime = Some("application/pdf".to_string());
    db.upsert_activity(&act101).unwrap();

    let bytes = db
        .export_course_package(&CourseExportSelection {
            course_id: 1,
            activity_ids: vec![101],
            include_section_ids: Vec::new(),
            include_files: false,
        })
        .unwrap();

    let mut archive = ZipArchive::new(Cursor::new(&bytes)).unwrap();
    assert_eq!(archive.len(), 1);
    let mut json_str = String::new();
    archive
        .by_name(ARCHIVE_MANIFEST_NAME)
        .unwrap()
        .read_to_string(&mut json_str)
        .unwrap();
    let backup: BackupData = serde_json::from_str(&json_str).unwrap();
    assert!(backup.files.is_empty());
    assert_eq!(backup.activities.len(), 1);
}

#[test]
fn export_empty_selection_returns_validation_error() {
    let db = Database::open_in_memory().unwrap();
    db.upsert_course(&sample_course(1, "Redes", false)).unwrap();

    let err = db
        .export_course_package(&CourseExportSelection {
            course_id: 1,
            activity_ids: Vec::new(),
            include_section_ids: Vec::new(),
            include_files: true,
        })
        .unwrap_err();
    assert!(err.to_string().contains("Selecione ao menos um item"));
}

#[test]
fn import_package_bytes_supports_plain_json_and_rejects_zip_slip() {
    let db1 = Database::open_in_memory().unwrap();
    db1.upsert_course(&sample_course(7, "Lógica", false))
        .unwrap();
    let json = db1.export_data_json().unwrap();

    let dst_cache = TempDir::new().unwrap();
    let db2 = Database::open_in_memory().unwrap();
    let json_summary = db2
        .import_package_bytes(json.as_bytes(), dst_cache.path())
        .unwrap();
    assert_eq!(json_summary.courses_imported, 1);
    assert_eq!(json_summary.files_imported, 0);

    // Monta um ZIP malicioso com path traversal em archive_path
    let mut bad_zip = ZipWriter::new(Cursor::new(Vec::new()));
    let manifest = serde_json::json!({
        "version": 2,
        "exported_at": Utc::now().to_rfc3339(),
        "courses": [],
        "sections": [],
        "activities": [],
        "outbox_items": [],
        "files": [{
            "activity_id": 1,
            "archive_path": "../evil.pdf",
            "file_name": "evil.pdf",
            "content_mime": "application/pdf",
            "size_bytes": 4
        }]
    })
    .to_string();
    bad_zip
        .start_file(ARCHIVE_MANIFEST_NAME, SimpleFileOptions::default())
        .unwrap();
    bad_zip.write_all(manifest.as_bytes()).unwrap();
    bad_zip
        .start_file("../evil.pdf", SimpleFileOptions::default())
        .unwrap();
    bad_zip.write_all(b"evil").unwrap();
    let bad_bytes = bad_zip.finish().unwrap().into_inner();

    let err = db2
        .import_package_bytes(&bad_bytes, dst_cache.path())
        .unwrap_err();
    assert!(err.to_string().contains("Caminho inseguro"));
}
