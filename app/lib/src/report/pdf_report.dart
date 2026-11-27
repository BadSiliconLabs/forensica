import 'dart:typed_data';

import 'package:pdf/pdf.dart';
import 'package:pdf/widgets.dart' as pw;

import '../../l10n/app_localizations.dart';
import '../rust/api/dto.dart';
import '../util/format.dart';

/// The built-in PDF fonts only cover Latin-1; map common typography and drop the rest.
String pdfSafe(String s) {
  const map = {
    '…': '...',
    '·': '-',
    '×': 'x',
    '→': '->',
    '←': '<-',
    '≤': '<=',
    '≥': '>=',
    'μ': 'u',
    '’': "'",
    '‘': "'",
    '“': '"',
    '”': '"',
    '—': '-',
    '–': '-',
    '•': '-',
    '≈': '~',
    '±': '+/-',
    '°': ' deg',
  };
  final b = StringBuffer();
  for (final r in s.runes) {
    final c = String.fromCharCode(r);
    if (map.containsKey(c)) {
      b.write(map[c]);
    } else if (r < 256) {
      b.write(c);
    } else {
      b.write('?');
    }
  }
  return b.toString();
}

/// Builds the PDF report.
Future<Uint8List> buildPdfReport({
  required SessionInfo session,
  required ReportDto report,
  required AppLocalizations l,
  Uint8List? previewPng,
  int maxVisualizations = 16,
}) async {
  final doc = pw.Document(
    title: pdfSafe('Forensica - ${session.name}'),
    creator: 'Forensica ${report.engineVersion}',
  );
  final findings = [for (final m in report.modules) ...m.findings]
    ..sort(
      (a, b) => severityRank(b.severity).compareTo(severityRank(a.severity)),
    );
  final viz = [for (final m in report.modules) ...m.visualizations]
      .take(maxVisualizations)
      .toList();
  String sev(String s) => switch (s) {
    'high' => l.severityHigh,
    'medium' => l.severityMedium,
    'low' => l.severityLow,
    _ => l.severityInfo,
  };
  PdfColor sevColor(String s) => switch (s) {
    'high' => PdfColors.red800,
    'medium' => PdfColors.orange800,
    'low' => PdfColors.blue800,
    _ => PdfColors.grey700,
  };
  int count(String s) => findings.where((f) => f.severity == s).length;
  pw.Widget kv(String k, String v) => pw.Padding(
    padding: const pw.EdgeInsets.symmetric(vertical: 1),
    child: pw.Row(
      crossAxisAlignment: pw.CrossAxisAlignment.start,
      children: [
        pw.SizedBox(
          width: 110,
          child: pw.Text(
            pdfSafe(k),
            style: pw.TextStyle(fontWeight: pw.FontWeight.bold, fontSize: 9),
          ),
        ),
        pw.Expanded(
          child: pw.Text(pdfSafe(v), style: const pw.TextStyle(fontSize: 9)),
        ),
      ],
    ),
  );

  doc.addPage(
    pw.MultiPage(
      pageFormat: PdfPageFormat.a4,
      margin: const pw.EdgeInsets.all(36),
      footer: (c) => pw.Align(
        alignment: pw.Alignment.centerRight,
        child: pw.Text(
          pdfSafe(
            'Forensica ${report.engineVersion} - SHA-256 ${report.sha256} - ${c.pageNumber}/${c.pagesCount}',
          ),
          style: const pw.TextStyle(fontSize: 7, color: PdfColors.grey600),
        ),
      ),
      build: (c) => [
        pw.Header(
          level: 0,
          child: pw.Text(
            pdfSafe('Forensica - ${l.findings}'),
            style: const pw.TextStyle(fontSize: 20),
          ),
        ),
        kv('File', session.name),
        kv(l.sha256, session.sha256),
        kv(l.format, '${session.format} (${session.mime})'),
        kv(l.size, formatBytes(session.size)),
        if (session.width > 0)
          kv(l.dimensions, '${session.width} x ${session.height}'),
        kv('Engine', 'Forensica ${report.engineVersion}'),
        kv('Analysed at', report.startedAt),
        kv('Duration', '${report.durationMs} ms'),
        pw.SizedBox(height: 10),
        pw.Container(
          padding: const pw.EdgeInsets.all(8),
          decoration: pw.BoxDecoration(
            color: PdfColors.grey200,
            borderRadius: pw.BorderRadius.circular(4),
          ),
          child: pw.Text(
            pdfSafe(l.disclaimer),
            style: const pw.TextStyle(fontSize: 9),
          ),
        ),
        pw.SizedBox(height: 10),
        pw.Text(
          pdfSafe(
            l.summaryCounts(
              count('high'),
              count('medium'),
              count('low'),
              count('info'),
            ),
          ),
          style: pw.TextStyle(fontWeight: pw.FontWeight.bold),
        ),
        pw.SizedBox(height: 8),
        if (findings.isEmpty) pw.Text(pdfSafe(l.noFindings)),
        for (final f in findings)
          pw.Container(
            margin: const pw.EdgeInsets.only(bottom: 8),
            padding: const pw.EdgeInsets.all(6),
            decoration: pw.BoxDecoration(
              border: pw.Border(
                left: pw.BorderSide(color: sevColor(f.severity), width: 3),
              ),
            ),
            child: pw.Column(
              crossAxisAlignment: pw.CrossAxisAlignment.start,
              children: [
                pw.Text(
                  pdfSafe(
                    '[${sev(f.severity)} - ${l.confidence((f.confidence * 100).round())}] ${f.title}',
                  ),
                  style: pw.TextStyle(
                    fontWeight: pw.FontWeight.bold,
                    fontSize: 10,
                    color: sevColor(f.severity),
                  ),
                ),
                if (f.explanation.isNotEmpty)
                  pw.Text(
                    pdfSafe(f.explanation),
                    style: const pw.TextStyle(fontSize: 9),
                  ),
                if (f.limitations.isNotEmpty)
                  pw.Text(
                    pdfSafe('${l.limitations}: ${f.limitations}'),
                    style: pw.TextStyle(
                      fontSize: 8,
                      fontStyle: pw.FontStyle.italic,
                      color: PdfColors.grey800,
                    ),
                  ),
                for (final e in f.evidence.take(12))
                  kv(
                    e.key,
                    e.value.length > 300
                        ? '${e.value.substring(0, 300)}...'
                        : e.value,
                  ),
                pw.Text(
                  pdfSafe('${f.moduleId} / ${f.code}'),
                  style: const pw.TextStyle(
                    fontSize: 7,
                    color: PdfColors.grey600,
                  ),
                ),
              ],
            ),
          ),
        if (viz.isNotEmpty) ...[
          pw.Header(level: 1, child: pw.Text(pdfSafe(l.visualizations))),
          for (final v in viz)
            pw.Container(
              margin: const pw.EdgeInsets.only(bottom: 12),
              child: pw.Column(
                crossAxisAlignment: pw.CrossAxisAlignment.start,
                children: [
                  pw.Text(
                    pdfSafe(v.title),
                    style: pw.TextStyle(fontWeight: pw.FontWeight.bold),
                  ),
                  pw.Text(
                    pdfSafe(v.description),
                    style: const pw.TextStyle(fontSize: 8),
                  ),
                  if (v.legendLow.isNotEmpty)
                    pw.Text(
                      pdfSafe(
                        '${l.legend}: ${v.legendLow} -> ${v.legendHigh} (${v.colormap})',
                      ),
                      style: const pw.TextStyle(fontSize: 8),
                    ),
                  pw.SizedBox(height: 4),
                  pw.Image(
                    pw.MemoryImage(v.png),
                    height: 260,
                    fit: pw.BoxFit.contain,
                  ),
                ],
              ),
            ),
        ],
        pw.Header(level: 1, child: pw.Text(pdfSafe(l.moduleTimes))),
        pw.TableHelper.fromTextArray(
          headers: ['Module', 'Version', 'Status', 'ms'],
          data: [
            for (final m in report.modules)
              [
                pdfSafe(m.name),
                '${m.version}',
                pdfSafe(m.status + (m.error != null ? ': ${m.error}' : '')),
                '${m.durationMs}',
              ],
          ],
          cellStyle: const pw.TextStyle(fontSize: 8),
          headerStyle: pw.TextStyle(
            fontSize: 8,
            fontWeight: pw.FontWeight.bold,
          ),
        ),
      ],
    ),
  );
  return doc.save();
}
