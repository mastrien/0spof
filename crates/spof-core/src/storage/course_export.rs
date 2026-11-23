use super::{BackupData, BundledFile, Database, ImportSummary};
use crate::error::{Result, SpofError};
use crate::models::{Activity, ActivityType, Course, Section};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

pub const ARCHIVE_MANIFEST_NAME: &str = "0spof.json";
const ZIP_LOCAL_HEADER_MAGIC: &[u8; 4] = b"PK\x03\x04";

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CourseExportSelection {
    pub course_id: i64,
    pub activity_ids: Vec<i64>,
    #[serde(default)]
    pub include_section_ids: Vec<i64>,
    #[serde(default = "default_true")]
    pub include_files: bool,
}

/// Indica se o item é um material de estudo (em vez de atividade avaliativa/interativa).
/// Na exportação seletiva para compartilhamento, materiais vêm pré-selecionados e atividades
/// (tarefas, questionários, fóruns) vêm desmarcadas por padrão.
pub fn is_material_activity(act: &Activity) -> bool {
    match &act.activity_type {
        ActivityType::Resource | ActivityType::Folder | ActivityType::Url => true,
        ActivityType::Other(modname) => matches!(
            modname.to_ascii_lowercase().as_str(),
            "page" | "book" | "lesson" | "label"
        ),
        ActivityType::Assign | ActivityType::Quiz | ActivityType::Forum => false,
    }
}

/// Identifica itens em PDF pelo MIME type, arquivo local, URL ou título do recurso.
pub fn is_pdf_activity(act: &Activity) -> bool {
    if act.content_mime.as_deref().is_some_and(|mime| {
        mime.split(';')
            .next()
            .unwrap_or("")
            .trim()
            .eq_ignore_ascii_case("application/pdf")
    }) {
        return true;
    }

    if act
        .local_file_path
        .as_deref()
        .is_some_and(|p| p.trim().to_ascii_lowercase().ends_with(".pdf"))
    {
        return true;
    }

    if act.url.as_deref().is_some_and(|u| {
        u.split(['?', '#'])
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase()
            .ends_with(".pdf")
    }) {
        return true;
    }

    act.activity_type == ActivityType::Resource
        && act.title.trim().to_ascii_lowercase().ends_with(".pdf")
}

fn transliterate_ascii(ch: char) -> char {
    match ch {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' | 'Å' => 'a',
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
        'ç' | 'Ç' => 'c',
        'ñ' | 'Ñ' => 'n',
        other => other,
    }
}

fn slugify_segment(input: &str, fallback: &str, max_len: usize) -> String {
    let mut out = String::new();
    let mut prev_dash = false;

    for raw in input.chars() {
        let ch = transliterate_ascii(raw);
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
        if out.len() >= max_len {
            break;
        }
    }

    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed
    }
}

fn sanitize_path_segment(input: &str, fallback: &str, max_len: usize) -> String {
    let mut cleaned = String::new();
    for raw in input.chars() {
        let ch = transliterate_ascii(raw);
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            cleaned.push(ch);
        } else {
            cleaned.push('_');
        }
        if cleaned.len() >= max_len {
            break;
        }
    }
    let trimmed = cleaned.trim_matches(['_', '.']);
    if trimmed.is_empty() || trimmed.contains("..") {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn sanitize_filename_preserving_ext(
    input: &str,
    fallback_stem: &str,
    max_stem_len: usize,
) -> String {
    let base = Path::new(input)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .trim();

    if let Some((stem, ext)) = base.rsplit_once('.') {
        let clean_ext: String = ext
            .chars()
            .map(transliterate_ascii)
            .filter(|c| c.is_ascii_alphanumeric())
            .take(10)
            .collect::<String>()
            .to_ascii_lowercase();
        if !clean_ext.is_empty() && !stem.trim().is_empty() {
            let clean_stem = sanitize_path_segment(stem, fallback_stem, max_stem_len);
            return format!("{clean_stem}.{clean_ext}");
        }
    }

    sanitize_path_segment(base, fallback_stem, max_stem_len)
}

/// Sugere um nome de arquivo `.zip` legível para exportação de uma disciplina.
pub fn suggested_course_export_filename(course: &Course, date: &str) -> String {
    let raw_name = course.fullname.trim();
    let stripped_prefix = if raw_name.starts_with('[') {
        raw_name
            .split_once(']')
            .map(|(_, rest)| rest.trim())
            .filter(|rest| !rest.is_empty())
            .unwrap_or(raw_name)
    } else {
        raw_name
    };

    let candidate = if !stripped_prefix.is_empty() {
        stripped_prefix
    } else if !course.shortname.trim().is_empty() {
        course.shortname.trim()
    } else {
        ""
    };

    let slug = slugify_segment(candidate, &format!("curso-{}", course.id), 48);
    let safe_date = sanitize_path_segment(date, "export", 20);
    format!("0spof-{slug}-{safe_date}.zip")
}

fn compression_for_filename(name: &str) -> CompressionMethod {
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(
        ext.as_str(),
        "pdf" | "png" | "jpg" | "jpeg" | "webp" | "gif" | "mp4" | "webm" | "mkv" | "zip" | "gz"
    ) {
        CompressionMethod::Stored
    } else {
        CompressionMethod::Deflated
    }
}

fn unique_archive_path(used: &mut HashSet<String>, dir: &str, file_name: &str) -> String {
    let candidate = format!("{dir}/{file_name}");
    if used.insert(candidate.clone()) {
        return candidate;
    }

    let (stem, ext) = match file_name.rsplit_once('.') {
        Some((s, e)) => (s, Some(e)),
        None => (file_name, None),
    };
    let mut counter = 2usize;
    loop {
        let next_name = match ext {
            Some(e) => format!("{stem}_{counter}.{e}"),
            None => format!("{stem}_{counter}"),
        };
        let next_path = format!("{dir}/{next_name}");
        if used.insert(next_path.clone()) {
            return next_path;
        }
        counter += 1;
    }
}

fn unique_dest_filename(used: &mut HashSet<String>, file_name: &str) -> String {
    if used.insert(file_name.to_string()) {
        return file_name.to_string();
    }
    let (stem, ext) = match file_name.rsplit_once('.') {
        Some((s, e)) => (s, Some(e)),
        None => (file_name, None),
    };
    let mut counter = 2usize;
    loop {
        let next_name = match ext {
            Some(e) => format!("{stem}_{counter}.{e}"),
            None => format!("{stem}_{counter}"),
        };
        if used.insert(next_name.clone()) {
            return next_name;
        }
        counter += 1;
    }
}

fn is_safe_archive_path(archive_path: &str) -> bool {
    if archive_path.is_empty()
        || archive_path.starts_with('/')
        || archive_path.starts_with('\\')
        || archive_path.contains('\\')
        || archive_path.contains('\0')
    {
        return false;
    }
    !Path::new(archive_path).components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

fn prefixed_activity_filename(activity_id: i64, raw_name: &str) -> String {
    let safe_base =
        sanitize_filename_preserving_ext(raw_name, &format!("arquivo_{activity_id}"), 64);
    let prefix = format!("{activity_id}_");
    if safe_base.starts_with(&prefix) {
        safe_base
    } else {
        format!("{prefix}{safe_base}")
    }
}

fn zip_err<E: std::fmt::Display>(err: E) -> SpofError {
    SpofError::Validation(format!("Erro no pacote ZIP: {err}"))
}

impl Database {
    /// Exporta uma única disciplina como pacote `.zip` compartilhável contendo apenas os itens
    /// selecionados e, opcionalmente, os arquivos offline baixados (PDFs, imagens, HTML).
    /// Caminhos locais e estado pessoal (`is_muted`, `is_submitted`, `outbox_items`) não são vazados.
    pub fn export_course_package(&self, selection: &CourseExportSelection) -> Result<Vec<u8>> {
        let mut course = self.get_course(selection.course_id)?.ok_or_else(|| {
            SpofError::Validation(format!(
                "Disciplina #{} não encontrada.",
                selection.course_id
            ))
        })?;
        course.is_muted = false;

        let selected_act_ids: HashSet<i64> = selection.activity_ids.iter().copied().collect();
        let selected_sec_ids: HashSet<i64> =
            selection.include_section_ids.iter().copied().collect();

        let sections = self.get_sections_by_course(selection.course_id)?;

        let mut exported_sections: Vec<Section> = Vec::new();
        let mut exported_activities: Vec<Activity> = Vec::new();
        let mut bundled_files: Vec<BundledFile> = Vec::new();
        let mut used_archive_paths: HashSet<String> = HashSet::new();

        let cursor = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(cursor);

        for (sec_idx, section) in sections.into_iter().enumerate() {
            let chosen_activities: Vec<Activity> = section
                .activities
                .into_iter()
                .filter(|act| selected_act_ids.contains(&act.id))
                .collect();

            let keep_empty_section =
                chosen_activities.is_empty() && selected_sec_ids.contains(&section.id);
            if chosen_activities.is_empty() && !keep_empty_section {
                continue;
            }

            let sec_slug = slugify_segment(&section.name, &format!("secao-{}", section.id), 36);
            let archive_dir = format!("materiais/{:02}_{sec_slug}", sec_idx + 1);

            for mut act in chosen_activities {
                if selection.include_files {
                    if let Some(ref local_path_str) = act.local_file_path {
                        let local_path = Path::new(local_path_str);
                        if !local_path_str.is_empty() && local_path.is_file() {
                            let bytes = std::fs::read(local_path)?;
                            let orig_name = local_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");
                            let file_name = prefixed_activity_filename(act.id, orig_name);
                            let archive_path = unique_archive_path(
                                &mut used_archive_paths,
                                &archive_dir,
                                &file_name,
                            );

                            let options = SimpleFileOptions::default()
                                .compression_method(compression_for_filename(&file_name));
                            zip.start_file(&archive_path, options).map_err(zip_err)?;
                            zip.write_all(&bytes)?;

                            bundled_files.push(BundledFile {
                                activity_id: act.id,
                                archive_path,
                                file_name,
                                content_mime: act.content_mime.clone(),
                                size_bytes: bytes.len() as u64,
                            });
                        }
                    }
                }

                act.local_file_path = None;
                act.is_submitted = false;
                exported_activities.push(act);
            }

            exported_sections.push(Section {
                id: section.id,
                course_id: section.course_id,
                name: section.name,
                summary: section.summary,
                activities: Vec::new(),
            });
        }

        if exported_sections.is_empty() {
            return Err(SpofError::Validation(
                "Selecione ao menos um item para exportar.".to_string(),
            ));
        }

        let manifest = BackupData {
            version: 2,
            exported_at: Utc::now(),
            courses: vec![course],
            sections: exported_sections,
            activities: exported_activities,
            outbox_items: Vec::new(),
            files: bundled_files,
        };
        let manifest_json = serde_json::to_string_pretty(&manifest)?;

        let manifest_opts =
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file(ARCHIVE_MANIFEST_NAME, manifest_opts)
            .map_err(zip_err)?;
        zip.write_all(manifest_json.as_bytes())?;

        let finished = zip.finish().map_err(zip_err)?;
        Ok(finished.into_inner())
    }

    /// Importa tanto backups completos (`.json`) quanto pacotes compartilhados de disciplina (`.zip`),
    /// extraindo arquivos anexados com segurança para `cache_dir` e vinculando `local_file_path`.
    pub fn import_package_bytes(&self, bytes: &[u8], cache_dir: &Path) -> Result<ImportSummary> {
        if !bytes.starts_with(ZIP_LOCAL_HEADER_MAGIC) {
            let json_str = std::str::from_utf8(bytes).map_err(|_| {
                SpofError::Validation(
                    "Arquivo inválido: esperado backup .json (UTF-8) ou pacote .zip.".to_string(),
                )
            })?;
            return self.import_data_json(json_str);
        }

        let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(zip_err)?;
        let manifest_json = {
            let mut manifest_file = archive.by_name(ARCHIVE_MANIFEST_NAME).map_err(|_| {
                SpofError::Validation(format!(
                    "Pacote ZIP inválido: '{ARCHIVE_MANIFEST_NAME}' não encontrado."
                ))
            })?;
            let mut buf = String::new();
            manifest_file.read_to_string(&mut buf)?;
            buf
        };

        let backup: BackupData = serde_json::from_str(&manifest_json)?;

        let mut extracted_files: HashMap<i64, (String, Option<String>)> = HashMap::new();
        let mut used_dest_names: HashSet<String> = HashSet::new();
        let mut files_imported = 0usize;

        if !backup.files.is_empty() {
            std::fs::create_dir_all(cache_dir)?;
            for bundled in &backup.files {
                if !is_safe_archive_path(&bundled.archive_path) {
                    return Err(SpofError::Validation(format!(
                        "Caminho inseguro no pacote ZIP: {}",
                        bundled.archive_path
                    )));
                }

                let file_bytes = {
                    let mut entry = archive.by_name(&bundled.archive_path).map_err(zip_err)?;
                    let mut data = Vec::new();
                    entry.read_to_end(&mut data)?;
                    data
                };

                let base_name = prefixed_activity_filename(bundled.activity_id, &bundled.file_name);
                let dest_name = unique_dest_filename(&mut used_dest_names, &base_name);
                let dest_path = cache_dir.join(dest_name);
                std::fs::write(&dest_path, &file_bytes)?;

                extracted_files.insert(
                    bundled.activity_id,
                    (
                        dest_path.to_string_lossy().to_string(),
                        bundled.content_mime.clone(),
                    ),
                );
                files_imported += 1;
            }
        }

        for course in &backup.courses {
            self.upsert_course(course)?;
        }
        for section in &backup.sections {
            self.upsert_section(section)?;
        }
        for activity in &backup.activities {
            let mut act = activity.clone();
            act.local_file_path = None;
            if let Some((path_str, mime)) = extracted_files.get(&act.id) {
                act.local_file_path = Some(path_str.clone());
                if mime.is_some() {
                    act.content_mime = mime.clone();
                }
            }
            self.upsert_activity(&act)?;
        }
        for item in &backup.outbox_items {
            let _ = self.insert_outbox_item(item)?;
        }

        Ok(ImportSummary {
            courses_imported: backup.courses.len(),
            sections_imported: backup.sections.len(),
            activities_imported: backup.activities.len(),
            outbox_items_imported: backup.outbox_items.len(),
            files_imported,
        })
    }

    pub fn import_package_file(&self, path: &Path, cache_dir: &Path) -> Result<ImportSummary> {
        let bytes = std::fs::read(path)?;
        self.import_package_bytes(&bytes, cache_dir)
    }
}
