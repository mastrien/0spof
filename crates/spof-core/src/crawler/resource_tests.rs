use super::*;

#[test]
fn test_classify_content_type() {
    assert_eq!(
        classify_content_type("application/pdf; charset=binary", "https://moodle/file"),
        MediaClassification::AutoDownload {
            mime: "application/pdf".to_string(),
            extension: "pdf".to_string()
        }
    );
    assert_eq!(
        classify_content_type("image/png", "https://moodle/img.png"),
        MediaClassification::AutoDownload {
            mime: "image/png".to_string(),
            extension: "png".to_string()
        }
    );
    assert_eq!(
        classify_content_type("video/mp4", "https://moodle/aula.mp4"),
        MediaClassification::ExplicitVideo {
            mime: "video/mp4".to_string(),
            extension: "mp4".to_string()
        }
    );
}

#[test]
fn test_classify_html_webm_and_other() {
    assert_eq!(
        classify_content_type("text/html; charset=utf-8", "https://moodle/mod/page"),
        MediaClassification::AutoDownload {
            mime: "text/html".to_string(),
            extension: "html".to_string()
        }
    );
    assert_eq!(
        classify_content_type("application/octet-stream", "https://moodle/aula.webm"),
        MediaClassification::ExplicitVideo {
            mime: "video/webm".to_string(),
            extension: "webm".to_string()
        }
    );
    assert_eq!(
        classify_content_type("image/jpeg", "https://moodle/foto"),
        MediaClassification::AutoDownload {
            mime: "image/jpeg".to_string(),
            extension: "jpg".to_string()
        }
    );
    assert_eq!(
        classify_content_type("", "https://moodle/arquivo.zip"),
        MediaClassification::Other {
            mime: "application/octet-stream".to_string(),
            extension: "bin".to_string()
        }
    );
}

#[test]
fn test_sanitize_filename() {
    assert_eq!(
        sanitize_filename("Slides: Aula 01/SQL", 2971, "pdf"),
        "2971_Slides__Aula_01_SQL.pdf"
    );
    assert_eq!(sanitize_filename("???", 7, "html"), "resource_7.html");
}

#[test]
fn test_extract_pluginfile_url_absent() {
    assert_eq!(extract_pluginfile_url("<p>Sem anexos</p>"), None);
}

#[test]
fn test_extract_pluginfile_url() {
    let html = r#"
        <div class="resourceworkaround">
            Clique em <a href="https://moodle/pluginfile.php/123/mod_resource/content/1/slides.pdf">slides.pdf</a>
        </div>
    "#;
    assert_eq!(
        extract_pluginfile_url(html),
        Some((
            "https://moodle/pluginfile.php/123/mod_resource/content/1/slides.pdf".to_string(),
            false
        ))
    );

    let video_html = r#"<video><source src="https://moodle/pluginfile.php/55/video.mp4" type="video/mp4"></video>"#;
    assert_eq!(
        extract_pluginfile_url(video_html),
        Some((
            "https://moodle/pluginfile.php/55/video.mp4".to_string(),
            true
        ))
    );
}

#[test]
fn test_extract_workaround_url() {
    let html_with_workaround = r#"
        <div class="urlworkaround">
            <a href="https://meet.google.com/abc-defg-hij">Clique para acessar a sala de aula virtual</a>
        </div>
    "#;
    assert_eq!(
        extract_workaround_url(html_with_workaround),
        Some("https://meet.google.com/abc-defg-hij".to_string())
    );

    let html_with_generic_moodle = r#"
        <div class="urlworkaround">
            <a href="https://moodle.edu.br/mod/url/view.php?id=99">Tentar novamente</a>
        </div>
    "#;
    assert_eq!(extract_workaround_url(html_with_generic_moodle), None);
}
