import 'dart:convert';
import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:material_ui/material_ui.dart';

import '../../l10n/app_localizations.dart';
import '../report/pdf_report.dart';
import '../rust/api/dto.dart';
import '../rust/api/engine.dart' as engine;

/// Linux batch mode: folder in, JSON + PDF reports out.
class BatchScreen extends StatefulWidget {
  const BatchScreen({super.key});

  @override
  State<BatchScreen> createState() => _BatchScreenState();
}

class _BatchScreenState extends State<BatchScreen> {
  String? _in;
  String? _out;
  bool _running = false;
  bool _cancel = false;
  int _done = 0;
  int _failed = 0;
  int _total = 0;
  final _log = <String>[];

  static const _exts = {'jpg', 'jpeg', 'png', 'webp', 'tif', 'tiff', 'heic', 'heif', 'avif', 'gif', 'bmp', 'dng'};

  Future<void> _run(AppLocalizations l) async {
    final inDir = _in, outDir = _out;
    if (inDir == null || outDir == null) return;
    final files = Directory(inDir)
        .listSync()
        .whereType<File>()
        .where((f) => _exts.contains(f.path.split('.').last.toLowerCase()))
        .toList()
      ..sort((a, b) => a.path.compareTo(b.path));
    setState(() {
      _running = true;
      _cancel = false;
      _done = 0;
      _failed = 0;
      _total = files.length;
      _log.clear();
    });
    for (final f in files) {
      if (_cancel) break;
      SessionInfo? s;
      try {
        s = await engine.openSession(path: f.path);
        ReportDto? report;
        await for (final e in engine.analyze(
          id: s.id,
          options: const AnalysisOptions(maxPixels: 0, elaQuality: 90, modules: [], extra: []),
        )) {
          if (e.report != null) report = e.report;
        }
        if (report == null) throw StateError('no report');
        final base = '$outDir/${s.name}';
        await File('$base.forensica.json').writeAsString(await engine.reportJson(id: s.id));
        await File('$base.forensica.pdf').writeAsBytes(await buildPdfReport(session: s, report: report, l: l));
        final high = [for (final m in report.modules) ...m.findings].where((x) => x.severity == 'high').length;
        setState(() {
          _done++;
          _log.add('✓ ${s!.name}  ($high high)');
        });
      } catch (e) {
        setState(() {
          _failed++;
          _log.add('✗ ${f.path}: $e');
        });
      } finally {
        if (s != null) engine.closeSession(id: s.id);
      }
    }
    setState(() {
      _running = false;
      _log.add(l.batchDone(_done, _failed));
    });
    // Keep the JSON tidy for scripts: write an index.
    final index = {'input': inDir, 'analysed': _done, 'failed': _failed, 'log': _log};
    await File('$outDir/forensica-batch-index.json').writeAsString(const JsonEncoder.withIndent('  ').convert(index));
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(l.batch)),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(l.batchHint),
            const SizedBox(height: 16),
            ListTile(
              leading: const Icon(Icons.folder_open),
              title: Text(l.chooseInputFolder),
              subtitle: Text(_in ?? '—'),
              onTap: _running
                  ? null
                  : () async {
                      // ignore: deprecated_member_use
                      final d = await FilePicker.getDirectoryPath();
                      if (d != null) setState(() => _in = d);
                    },
            ),
            ListTile(
              leading: const Icon(Icons.drive_folder_upload),
              title: Text(l.chooseOutputFolder),
              subtitle: Text(_out ?? '—'),
              onTap: _running
                  ? null
                  : () async {
                      // ignore: deprecated_member_use
                      final d = await FilePicker.getDirectoryPath();
                      if (d != null) setState(() => _out = d);
                    },
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                FilledButton.icon(
                  onPressed: (_running || _in == null || _out == null) ? null : () => _run(l),
                  icon: const Icon(Icons.play_arrow),
                  label: Text(l.start),
                ),
                const SizedBox(width: 12),
                if (_running) OutlinedButton(onPressed: () => setState(() => _cancel = true), child: Text(l.cancel)),
              ],
            ),
            const SizedBox(height: 12),
            if (_total > 0) LinearProgressIndicator(value: (_done + _failed) / _total),
            const SizedBox(height: 12),
            Expanded(
              child: ListView(children: [for (final line in _log) Text(line, style: const TextStyle(fontFamily: 'monospace'))]),
            ),
          ],
        ),
      ),
    );
  }
}
