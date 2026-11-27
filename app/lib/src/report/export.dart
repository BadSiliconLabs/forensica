import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';

import '../../l10n/app_localizations.dart';
import '../rust/api/engine.dart' as engine;
import '../state/analysis.dart';
import 'pdf_report.dart';

String _baseName(String name) {
  final i = name.lastIndexOf('.');
  return i > 0 ? name.substring(0, i) : name;
}

Future<String?> _save(String fileName, Uint8List bytes, String mime) async {
  final uri = await FilePicker.saveFile(fileName: fileName, bytes: bytes, mimeType: mime);
  if (uri == null) return null;
  if (uri.scheme == 'file') {
    final f = File(uri.toFilePath());
    // Some platforms only return a path; make sure the bytes are there.
    if (!await f.exists() || await f.length() == 0) await f.writeAsBytes(bytes);
    return f.path;
  }
  return uri.toString();
}

/// Exports the PDF report through a save dialog. Returns the destination or null if cancelled.
Future<String?> exportPdf(AnalysisState a, AppLocalizations l) async {
  final s = a.session;
  final r = a.report;
  if (s == null || r == null) return null;
  final bytes = await buildPdfReport(session: s, report: r, l: l);
  return _save('${_baseName(s.name)}.forensica.pdf', bytes, 'application/pdf');
}

/// Exports the JSON report.
Future<String?> exportJson(AnalysisState a) async {
  final s = a.session;
  if (s == null) return null;
  final json = await engine.reportJson(id: s.id);
  return _save('${_baseName(s.name)}.forensica.json', Uint8List.fromList(utf8.encode(json)), 'application/json');
}
