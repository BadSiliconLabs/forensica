import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:file_picker/file_picker.dart';
import 'package:path_provider/path_provider.dart';

import '../../l10n/app_localizations.dart';
import '../rust/api/engine.dart' as engine;
import '../state/analysis.dart';
import 'pdf_report.dart';

String _baseName(String name) {
  final i = name.lastIndexOf('.');
  return i > 0 ? name.substring(0, i) : name;
}

Future<String?> _save(String fileName, Uint8List bytes, String mime) async {
  Uri? uri;
  try {
    uri = await FilePicker.saveFile(
      fileName: fileName,
      bytes: bytes,
      mimeType: mime,
    );
  } catch (_) {
    // No save dialog available (e.g. Linux without zenity/kdialog/portal):
    // fall back to the Downloads folder.
    if (!Platform.isLinux) rethrow;
    final dir = await getDownloadsDirectory() ??
        Directory(Platform.environment['HOME'] ?? '.');
    final f = File('${dir.path}/$fileName');
    await f.writeAsBytes(bytes);
    return f.path;
  }
  if (uri == null) return null;
  if (uri.scheme == 'file') {
    final f = File(uri.toFilePath());
    // Some platforms only return a path; make sure the bytes are there.
    if (!await f.exists() || await f.length() == 0) await f.writeAsBytes(bytes);
    return f.path;
  }
  return uri.toString();
}

/// Encodes [img] as PNG with its longest side limited to [maxSide].
Future<Uint8List?> _scaledPng(ui.Image img, int maxSide) async {
  final k = maxSide / (img.width > img.height ? img.width : img.height);
  var src = img;
  if (k < 1) {
    final w = (img.width * k).round(), h = (img.height * k).round();
    final rec = ui.PictureRecorder();
    ui.Canvas(rec).drawImageRect(
      img,
      ui.Rect.fromLTWH(0, 0, img.width.toDouble(), img.height.toDouble()),
      ui.Rect.fromLTWH(0, 0, w.toDouble(), h.toDouble()),
      ui.Paint()..filterQuality = ui.FilterQuality.medium,
    );
    src = await rec.endRecording().toImage(w, h);
  }
  final data = await src.toByteData(format: ui.ImageByteFormat.png);
  if (!identical(src, img)) src.dispose();
  return data?.buffer.asUint8List();
}

/// Exports the PDF report through a save dialog. Returns the destination or null if cancelled.
Future<String?> exportPdf(AnalysisState a, AppLocalizations l) async {
  final s = a.session;
  final r = a.report;
  if (s == null || r == null) return null;
  Uint8List? preview;
  final img = a.preview;
  if (img != null) preview = await _scaledPng(img, 1200);
  final bytes = await buildPdfReport(
    session: s,
    report: r,
    l: l,
    previewPng: preview,
  );
  return _save('${_baseName(s.name)}.forensica.pdf', bytes, 'application/pdf');
}

/// Exports the JSON report.
Future<String?> exportJson(AnalysisState a) async {
  final s = a.session;
  if (s == null) return null;
  final json = await engine.reportJson(id: s.id);
  return _save(
    '${_baseName(s.name)}.forensica.json',
    Uint8List.fromList(utf8.encode(json)),
    'application/json',
  );
}
