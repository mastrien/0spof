import 'dart:convert';

/// Modelos espelhando os DTOs de `spof_core::api::dto`. O bridge devolve JSON.
List<T> _list<T>(String raw, T Function(Map<String, dynamic>) f) =>
    (jsonDecode(raw) as List).map((e) => f(e as Map<String, dynamic>)).toList();

class SystemStatus {
  final int courses, activeCourses, mutedCourses, deadlines, outbox;
  SystemStatus.fromJson(Map<String, dynamic> j)
      : courses = j['courses_count'],
        activeCourses = j['active_courses_count'],
        mutedCourses = j['muted_courses_count'],
        deadlines = j['deadlines_count'],
        outbox = j['outbox_count'];
  static SystemStatus parse(String raw) => SystemStatus.fromJson(jsonDecode(raw));
}

class Course {
  final int id, sections, activities;
  final String fullname, shortname;
  final bool muted;
  Course.fromJson(Map<String, dynamic> j)
      : id = j['id'],
        fullname = j['fullname'],
        shortname = j['shortname'],
        sections = j['sections_count'],
        activities = j['activities_count'],
        muted = j['is_muted'];
  static List<Course> parseList(String raw) => _list(raw, Course.fromJson);
}

class Activity {
  final int id, sectionId;
  final String title, type;
  final String? url, description, localFilePath, mime;
  final DateTime? due;
  final bool submitted;
  Activity.fromJson(Map<String, dynamic> j)
      : id = j['id'],
        sectionId = j['section_id'],
        title = j['title'],
        type = j['activity_type'],
        url = j['url'],
        description = j['description'],
        localFilePath = j['local_file_path'],
        mime = j['content_mime'],
        due = j['due_date'] == null ? null : DateTime.parse(j['due_date']).toLocal(),
        submitted = j['is_submitted'];
}

class Section {
  final int id;
  final String name;
  final String? summary;
  final List<Activity> activities;
  Section.fromJson(Map<String, dynamic> j)
      : id = j['id'],
        name = j['name'],
        summary = j['summary'],
        activities = (j['activities'] as List)
            .map((a) => Activity.fromJson(a as Map<String, dynamic>))
            .toList();
  static List<Section> parseList(String raw) => _list(raw, Section.fromJson);
}

class Deadline {
  final int id, sectionId;
  final String title, type;
  final String? url;
  final DateTime? due;
  final bool submitted;
  Deadline.fromJson(Map<String, dynamic> j)
      : id = j['id'],
        sectionId = j['section_id'],
        title = j['title'],
        type = j['activity_type'],
        url = j['url'],
        due = j['due_date'] == null ? null : DateTime.parse(j['due_date']).toLocal(),
        submitted = j['is_submitted'];
  static List<Deadline> parseList(String raw) => _list(raw, Deadline.fromJson);
}

class OutboxItem {
  final int id, assignId, retries;
  final String filename, hash, status;
  final bool hasToken;
  OutboxItem.fromJson(Map<String, dynamic> j)
      : id = j['id'],
        assignId = j['assign_id'],
        filename = j['filename'],
        hash = j['file_hash'],
        status = j['status'],
        retries = j['retry_count'],
        hasToken = j['has_rfc3161_token'];
  static List<OutboxItem> parseList(String raw) => _list(raw, OutboxItem.fromJson);
}

class SessionInfo {
  final bool authenticated;
  final String? baseUrl;
  SessionInfo.fromJson(Map<String, dynamic> j)
      : authenticated = j['is_authenticated'],
        baseUrl = j['base_url'];
  static SessionInfo parse(String raw) => SessionInfo.fromJson(jsonDecode(raw));
}

class SyncSummary {
  final int courses, sections, activities, resources;
  SyncSummary.fromJson(Map<String, dynamic> j)
      : courses = j['courses_synced'],
        sections = j['sections_synced'],
        activities = j['activities_synced'],
        resources = j['resources_cached'];
  static SyncSummary parse(String raw) => SyncSummary.fromJson(jsonDecode(raw));
}

class EnqueueResult {
  final int outboxId;
  final String hash;
  final bool hasTsaToken;
  EnqueueResult.fromJson(Map<String, dynamic> j)
      : outboxId = j['outbox_id'],
        hash = j['file_hash'],
        hasTsaToken = j['has_tsa_token'];
  static EnqueueResult parse(String raw) => EnqueueResult.fromJson(jsonDecode(raw));
}

class Proof {
  final int assignId;
  final String filename, hash, report;
  Proof.fromJson(Map<String, dynamic> j)
      : assignId = j['assign_id'],
        filename = j['filename'],
        hash = j['file_hash'],
        report = j['formatted_report'];
  static Proof parse(String raw) => Proof.fromJson(jsonDecode(raw));
}

class ExportItemPreview {
  final int id, sectionId;
  final String title, activityType;
  final bool isMaterial, selectedByDefault, isPdf, hasLocalFile;
  final String? fileName, contentMime, url;
  final int? fileSizeBytes;
  final DateTime? dueDate;

  ExportItemPreview.fromJson(Map<String, dynamic> j)
      : id = j['id'],
        sectionId = j['section_id'],
        title = j['title'],
        activityType = j['activity_type'],
        isMaterial = j['is_material'] ?? false,
        selectedByDefault = j['selected_by_default'] ?? false,
        isPdf = j['is_pdf'] ?? false,
        hasLocalFile = j['has_local_file'] ?? false,
        fileName = j['file_name'],
        fileSizeBytes = j['file_size_bytes'],
        contentMime = j['content_mime'],
        url = j['url'],
        dueDate = j['due_date'] == null ? null : DateTime.parse(j['due_date']).toLocal();
}

class ExportSectionPreview {
  final int id;
  final String name;
  final String? summary;
  final List<ExportItemPreview> items;

  ExportSectionPreview.fromJson(Map<String, dynamic> j)
      : id = j['id'],
        name = j['name'],
        summary = j['summary'],
        items = (j['items'] as List)
            .map((e) => ExportItemPreview.fromJson(e as Map<String, dynamic>))
            .toList();
}

class CourseExportPreview {
  final int courseId;
  final String fullname, shortname, suggestedFilename;
  final List<ExportSectionPreview> sections;

  CourseExportPreview.fromJson(Map<String, dynamic> j)
      : courseId = j['course_id'],
        fullname = j['fullname'],
        shortname = j['shortname'],
        suggestedFilename = j['suggested_filename'],
        sections = (j['sections'] as List)
            .map((e) => ExportSectionPreview.fromJson(e as Map<String, dynamic>))
            .toList();

  static CourseExportPreview parse(String raw) =>
      CourseExportPreview.fromJson(jsonDecode(raw) as Map<String, dynamic>);
}
