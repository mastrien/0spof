//! Busca global local: disciplinas, atividades e recursos por título ou sigla.
//!
//! Sem acento e sem diferenciar maiúsculas; todas as palavras digitadas precisam aparecer.

use super::dto::SearchHitDto;
use crate::storage::{is_material_activity, Database};

pub const DEFAULT_SEARCH_LIMIT: usize = 30;
const MAX_SEARCH_LIMIT: usize = 100;

/// Minúsculas e sem acentos do português.
pub fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

fn matches(haystack: &str, tokens: &[String]) -> bool {
    let hay = fold(haystack);
    tokens.iter().all(|t| hay.contains(t.as_str()))
}

/// Busca em disciplinas, atividades e recursos. `limit` é limitado a 100.
pub fn search(
    database: &Database,
    query: &str,
    limit: Option<usize>,
) -> Result<Vec<SearchHitDto>, String> {
    let tokens: Vec<String> = fold(query).split_whitespace().map(str::to_string).collect();
    if tokens.is_empty() {
        return Ok(Vec::new());
    }
    let limit = limit.unwrap_or(DEFAULT_SEARCH_LIMIT).min(MAX_SEARCH_LIMIT);
    let first = tokens[0].clone();

    // (prioridade, hit): 0 = começa com a busca, 1 = contém.
    let mut scored: Vec<(u8, SearchHitDto)> = Vec::new();
    for course in database.get_courses().map_err(|e| e.to_string())? {
        if matches(
            &format!("{} {}", course.fullname, course.shortname),
            &tokens,
        ) {
            let prio = u8::from(!fold(&course.fullname).starts_with(&first));
            scored.push((
                prio,
                SearchHitDto {
                    kind: "course".into(),
                    id: course.id,
                    title: course.fullname.clone(),
                    subtitle: course.shortname.clone(),
                    course_id: course.id,
                    course_short: course.shortname.clone(),
                    activity_type: None,
                    url: None,
                },
            ));
        }
    }
    // atividades: uma única consulta com JOIN (sem uma consulta por disciplina)
    for row in database.list_activity_index().map_err(|e| e.to_string())? {
        let act = row.activity;
        if !matches(&format!("{} {}", act.title, row.course_short), &tokens) {
            continue;
        }
        let prio = u8::from(!fold(&act.title).starts_with(&first));
        let kind = if is_material_activity(&act) {
            "resource"
        } else {
            "activity"
        };
        scored.push((
            prio,
            SearchHitDto {
                kind: kind.into(),
                id: act.id,
                title: act.title,
                subtitle: format!("{} · {}", row.course_short, row.section_name),
                course_id: row.course_id,
                course_short: row.course_short,
                activity_type: Some(act.activity_type.as_str().to_string()),
                url: act.url,
            },
        ));
    }

    let kind_rank = |k: &str| match k {
        "course" => 0,
        "activity" => 1,
        _ => 2,
    };
    scored.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| kind_rank(&a.1.kind).cmp(&kind_rank(&b.1.kind)))
            .then_with(|| a.1.title.cmp(&b.1.title))
    });
    Ok(scored.into_iter().take(limit).map(|(_, h)| h).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Activity, ActivityType, Course, Section};

    fn act(id: i64, title: &str, ty: ActivityType) -> Activity {
        Activity {
            id,
            section_id: 10,
            title: title.into(),
            activity_type: ty,
            url: Some(format!("https://m/{id}")),
            description: None,
            local_file_path: None,
            content_mime: None,
            due_date: None,
            is_submitted: false,
        }
    }

    fn seeded() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.upsert_course(&Course {
            id: 1,
            fullname: "Banco de Dados II".into(),
            shortname: "BDD2".into(),
            summary: None,
            last_synced_at: None,
            is_muted: false,
        })
        .unwrap();
        db.upsert_section(&Section {
            id: 10,
            course_id: 1,
            name: "Modelo relacional".into(),
            summary: None,
            activities: vec![],
        })
        .unwrap();
        db.upsert_activity(&act(
            100,
            "Modelagem relacional — Entrega 1",
            ActivityType::Assign,
        ))
        .unwrap();
        db.upsert_activity(&act(101, "Apostila de Álgebra", ActivityType::Resource))
            .unwrap();
        db.upsert_activity(&act(102, "Fórum de dúvidas", ActivityType::Forum))
            .unwrap();
        db
    }

    #[test]
    fn fold_removes_accents_and_case() {
        assert_eq!(fold("Álgebra Relacional ÇÃO"), "algebra relacional cao");
    }

    #[test]
    fn empty_query_returns_nothing() {
        let db = seeded();
        assert!(search(&db, "   ", None).unwrap().is_empty());
    }

    #[test]
    fn finds_course_by_shortname_and_name() {
        let db = seeded();
        let hits = search(&db, "bdd2", None).unwrap();
        assert_eq!(hits[0].kind, "course");
        assert_eq!(hits[0].course_id, 1);
        // a sigla também casa as atividades da disciplina
        assert!(hits.len() > 1);
        let by_name = search(&db, "banco dados", None).unwrap();
        assert_eq!(by_name[0].title, "Banco de Dados II");
    }

    #[test]
    fn is_accent_insensitive_and_classifies_kinds() {
        let db = seeded();
        let hits = search(&db, "algebra", None).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, "resource");
        assert_eq!(hits[0].activity_type.as_deref(), Some("resource"));
        assert_eq!(hits[0].subtitle, "BDD2 · Modelo relacional");
        assert_eq!(hits[0].url.as_deref(), Some("https://m/101"));

        let forum = search(&db, "FORUM", None).unwrap();
        assert_eq!(forum[0].kind, "activity");
    }

    #[test]
    fn all_tokens_must_match() {
        let db = seeded();
        assert!(search(&db, "modelagem inexistente", None)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn titles_starting_with_the_term_rank_before_titles_that_only_contain_it() {
        let db = seeded();
        // id menor e título em ordem alfabética anterior, mas só CONTÉM "relacional"
        db.upsert_activity(&act(
            1,
            "Aula sobre modelo relacional",
            ActivityType::Resource,
        ))
        .unwrap();
        // começa com o termo
        db.upsert_activity(&act(
            900,
            "Relacional: lista de exercícios",
            ActivityType::Resource,
        ))
        .unwrap();
        let hits = search(&db, "relacional", None).unwrap();
        let pos = |id: i64| hits.iter().position(|h| h.id == id).unwrap();
        assert!(
            pos(900) < pos(1),
            "prefixo deve vir antes: {:?}",
            hits.iter().map(|h| &h.title).collect::<Vec<_>>()
        );
        assert!(pos(900) < pos(100), "e antes do que só contém na seção");
    }

    #[test]
    fn limit_is_applied_and_capped() {
        let db = seeded();
        assert_eq!(search(&db, "bdd2", Some(2)).unwrap().len(), 2);
        assert!(search(&db, "bdd2", Some(10_000)).unwrap().len() <= MAX_SEARCH_LIMIT);
    }
}
