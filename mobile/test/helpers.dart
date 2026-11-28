import 'package:flutter/material.dart';
import 'package:spof_mobile_app/models.dart';
import 'package:spof_mobile_app/store.dart';
import 'package:spof_mobile_app/theme.dart';

Deadline deadline(int id, String? due, {bool submitted = false, String type = 'assign', String title = 'Tarefa'}) =>
    Deadline.fromJson({
      'id': id,
      'section_id': 1,
      'title': '$title $id',
      'activity_type': type,
      'url': null,
      'due_date': due,
      'is_submitted': submitted,
    });

OutboxItem outboxItem(int id, String status, {bool token = false, int assignId = 1, int retries = 0}) =>
    OutboxItem.fromJson({
      'id': id,
      'assign_id': assignId,
      'filename': 'arquivo_$id.pdf',
      'file_hash': '9f2c41ab0000000000000000000000000000000000000000000000000071d42ac',
      'status': status,
      'retry_count': retries,
      'has_rfc3161_token': token,
    });

Course course(int id, {bool muted = false, String name = 'Banco de Dados II', String short = 'BDD2'}) => Course.fromJson({
      'id': id,
      'fullname': name,
      'shortname': short,
      'sections_count': 3,
      'activities_count': 9,
      'is_muted': muted,
    });

/// Prévia de exportação com um item (PDF baixado, 1,2 MB) e, opcionalmente, uma atividade.
CourseExportPreview exportPreview({bool withActivity = false}) => CourseExportPreview.fromJson({
      'course_id': 1,
      'fullname': 'Banco de Dados II',
      'shortname': 'BDD2',
      'suggested_filename': 'x.zip',
      'sections': [
        {
          'id': 10,
          'name': 'Semana 1',
          'summary': null,
          'items': [
            {
              'id': 1,
              'section_id': 10,
              'title': 'Apostila',
              'activity_type': 'resource',
              'is_material': true,
              'selected_by_default': true,
              'is_pdf': true,
              'has_local_file': true,
              'file_name': 'aula.pdf',
              'file_size_bytes': 1258291,
              'content_mime': 'application/pdf',
              'url': null,
              'due_date': null,
            },
            if (withActivity)
              {
                'id': 2,
                'section_id': 10,
                'title': 'Questionário 1',
                'activity_type': 'quiz',
                'is_material': false,
                'selected_by_default': false,
                'is_pdf': false,
                'has_local_file': false,
                'file_name': null,
                'file_size_bytes': null,
                'content_mime': null,
                'url': null,
                'due_date': null,
              },
          ],
        },
      ],
    });

class FakeStore extends Store {
  final calls = <({int assignId, String path, bool fetchTsa})>[];
  CourseExportPreview? preview;

  @override
  Future<EnqueueResult> enqueueSubmission(int assignId, String pickedPath, {bool fetchTsa = true}) async {
    calls.add((assignId: assignId, path: pickedPath, fetchTsa: fetchTsa));
    return EnqueueResult.fromJson({'outbox_id': 7, 'file_hash': 'h', 'has_tsa_token': fetchTsa});
  }

  @override
  Future<CourseExportPreview> courseExportPreview(int courseId) async => preview ?? exportPreview();

  /// Seções devolvidas por `courseDetails` (o banco real fica no Rust).
  List<Section> sections = [];

  @override
  Future<List<Section>> courseDetails(int id) async => sections;

  @override
  Future<String> savedMoodleUrl() async => '';

  @override
  Future<void> pairWithPc(String code) async => throw Exception('código inválido');
}

Widget app(Widget home, {Brightness b = Brightness.light, double scale = 1}) => MaterialApp(
      theme: buildTheme(b),
      builder: (context, child) => MediaQuery(
        data: MediaQuery.of(context).copyWith(textScaler: TextScaler.linear(scale)),
        child: child!,
      ),
      home: home,
    );

/// Contraste WCAG entre duas cores.
double contrastRatio(Color a, Color b) {
  final la = a.computeLuminance(), lb = b.computeLuminance();
  final hi = la > lb ? la : lb, lo = la > lb ? lb : la;
  return (hi + 0.05) / (lo + 0.05);
}

/// Data/hora local relativa a agora (ISO sem fuso), para não depender do calendário real.
String relIso(Duration d) => DateTime.now().add(d).toIso8601String();

Section sectionOf(int id, String name, List<Map<String, dynamic>> acts) => Section.fromJson({
      'id': id,
      'name': name,
      'summary': null,
      'activities': acts,
    });

Map<String, dynamic> actJson(int id, String title, String type,
        {String? mime, String? localPath, String? due, bool submitted = false}) =>
    {
      'id': id,
      'section_id': 1,
      'title': title,
      'activity_type': type,
      'url': null,
      'description': null,
      'local_file_path': localPath,
      'content_mime': mime,
      'due_date': due,
      'is_submitted': submitted,
    };
