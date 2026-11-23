import 'package:flutter_test/flutter_test.dart';
import 'package:spof_mobile_app/course_export_page.dart';
import 'package:spof_mobile_app/models.dart';

void main() {
  const sampleJson = '''
  {
    "course_id": 42,
    "fullname": "Algoritmos e Estruturas",
    "shortname": "ALG",
    "suggested_filename": "0spof-algoritmos-e-estruturas-2026-11-22.zip",
    "sections": [
      {
        "id": 100,
        "name": "Semana 1",
        "summary": null,
        "items": [
          {
            "id": 1,
            "section_id": 100,
            "title": "Apostila 01",
            "activity_type": "resource",
            "is_material": true,
            "selected_by_default": true,
            "is_pdf": true,
            "has_local_file": true,
            "file_name": "1_Apostila_01.pdf",
            "file_size_bytes": 1572864,
            "content_mime": "application/pdf",
            "url": "https://moodle.test/r/1",
            "due_date": null
          },
          {
            "id": 2,
            "section_id": 100,
            "title": "Slides HTML",
            "activity_type": "page",
            "is_material": true,
            "selected_by_default": true,
            "is_pdf": false,
            "has_local_file": false,
            "file_name": null,
            "file_size_bytes": null,
            "content_mime": "text/html",
            "url": "https://moodle.test/page/2",
            "due_date": null
          },
          {
            "id": 3,
            "section_id": 100,
            "title": "Lista de Exercícios 1",
            "activity_type": "assign",
            "is_material": false,
            "selected_by_default": false,
            "is_pdf": false,
            "has_local_file": false,
            "file_name": null,
            "file_size_bytes": null,
            "content_mime": null,
            "url": "https://moodle.test/assign/3",
            "due_date": "2026-11-30T23:59:00Z"
          }
        ]
      }
    ]
  }
  ''';

  test('CourseExportPreview interpreta materiais por padrão e exclui atividades', () {
    final preview = CourseExportPreview.parse(sampleJson);
    expect(preview.courseId, 42);
    expect(preview.sections.length, 1);

    final defaults = defaultSelectedIds(preview);
    expect(defaults, {1, 2});
    expect(defaults.contains(3), isFalse);

    final onlyPdfs = applyExportPreset(preview, ExportPreset.pdfs);
    expect(onlyPdfs, {1});

    final all = applyExportPreset(preview, ExportPreset.all);
    expect(all, {1, 2, 3});

    final stats = computeExportStats(preview, defaults, includeFiles: true);
    expect(stats.selectedCount, 2);
    expect(stats.totalCount, 3);
    expect(stats.pdfCount, 1);
    expect(stats.totalBytes, 1572864);

    final statsNoFiles = computeExportStats(preview, defaults, includeFiles: false);
    expect(statsNoFiles.totalBytes, 0);
  });

  test('formatFileSize formata bytes e megabytes', () {
    expect(formatFileSize(0), '0 B');
    expect(formatFileSize(512), '512 B');
    expect(formatFileSize(1536), '1,5 KB');
    expect(formatFileSize(1572864), '1,5 MB');
  });
}
