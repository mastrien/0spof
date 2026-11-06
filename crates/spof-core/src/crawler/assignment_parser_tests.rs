use super::*;

#[test]
fn test_parse_moodle_date_ptbr() {
    let raw = "quarta, 12 nov. 2026, 23:59";
    let dt = parse_moodle_date(raw).expect("should parse PT-BR date with abbreviated month");
    // 2026-11-12 23:59:00 in UTC-3 -> 2026-11-13 02:59:00 UTC
    assert_eq!(dt.to_rfc3339(), "2026-11-13T02:59:00+00:00");

    let raw2 = "15/10/2026 18:30";
    let dt2 = parse_moodle_date(raw2).expect("should parse numeric slash date");
    assert_eq!(dt2.to_rfc3339(), "2026-10-15T21:30:00+00:00");

    let raw3 = "sexta-feira, 5 de dezembro de 2026, 23:59";
    let dt3 = parse_moodle_date(raw3).expect("should parse full month name");
    assert_eq!(dt3.to_rfc3339(), "2026-12-06T02:59:00+00:00");
}

#[test]
fn test_parse_assignment_details_submitted() {
    let html = r#"
        <html>
            <h2>Atividade Prática 2</h2>
            <div class="submissionstatustable">
                <table class="generaltable">
                    <tr>
                        <th>Status de envio</th>
                        <td>Submetido para avaliação</td>
                    </tr>
                    <tr>
                        <th>Data de entrega</th>
                        <td>quarta, 12 nov. 2026, 23:59</td>
                    </tr>
                    <tr>
                        <th>Tempo restante</th>
                        <td>A tarefa foi enviada 1 hora adiantada</td>
                    </tr>
                </table>
            </div>
            <div id="intro" class="intro">
                <p>Favor enviar o arquivo PDF contendo o relatório.</p>
                <p>Consulte as orientações em <a href="https://example.com/guide">Guia</a>.</p>
            </div>
        </html>
    "#;

    let details = parse_assignment_details(html, 999);
    assert_eq!(details.assign_id, 999);
    assert_eq!(details.title, "Atividade Prática 2");
    assert!(details.is_submitted);
    assert!(details.due_date.is_some());
    assert_eq!(
        details.time_remaining.as_deref(),
        Some("A tarefa foi enviada 1 hora adiantada")
    );
    assert!(details.description_html.is_some());
    let desc = details.description_html.unwrap();
    assert!(desc.contains("<a href=\"https://example.com/guide\">Guia</a>"));
    assert!(desc.contains("<p>Favor enviar o arquivo PDF contendo o relatório.</p>"));
}

#[test]
fn test_parse_assignment_details_unsubmitted() {
    let html = r#"
        <html>
            <h2>Lista de Exercícios 1</h2>
            <table>
                <tr>
                    <th>Status do envio</th>
                    <td>Nenhum envio</td>
                </tr>
                <tr>
                    <th>Data limite</th>
                    <td>20/11/2026 23:59</td>
                </tr>
                <tr>
                    <th>Tempo restante</th>
                    <td>5 dias restantes</td>
                </tr>
            </table>
        </html>
    "#;

    let details = parse_assignment_details(html, 555);
    assert_eq!(details.assign_id, 555);
    assert_eq!(details.title, "Lista de Exercícios 1");
    assert!(!details.is_submitted);
    assert!(details.due_date.is_some());
    assert_eq!(details.time_remaining.as_deref(), Some("5 dias restantes"));
}

#[test]
fn test_clean_html_strips_scripts() {
    let raw = "<p>Olá <strong>mundo</strong></p><script>alert('xss');</script>";
    let cleaned = clean_html(raw);
    assert_eq!(cleaned, "<p>Olá <strong>mundo</strong></p>");
}
