import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forensica/src/report/pdf_report.dart';
import 'package:forensica/src/rust/api/dto.dart';
import 'package:forensica/src/rust/api/engine.dart' as engine;
import 'package:forensica/src/rust/frb_generated.dart';
import 'package:forensica/src/state/settings.dart';
import 'package:forensica/src/util/format.dart';
import 'package:forensica/l10n/app_localizations_en.dart';

/// Path of the Rust library built by `cargo build -p forensica-bridge`.
String? _bridgeLib() {
  for (final p in ['../core/target/debug/libforensica_bridge.so', '../core/target/release/libforensica_bridge.so']) {
    if (File(p).existsSync()) return p;
  }
  return null;
}

void main() {
  group('format', () {
    test('formatBytes', () {
      expect(formatBytes(512), '512 B');
      expect(formatBytes(2048), '2.0 KB');
      expect(formatBytes(5 * 1024 * 1024), '5.0 MB');
    });
    test('shortSha', () {
      expect(shortSha('a' * 64), '${'a' * 8}…${'a' * 8}');
      expect(shortSha('abc'), 'abc');
    });
    test('severityRank orders', () {
      expect(severityRank('high') > severityRank('medium'), isTrue);
      expect(severityRank('info'), 0);
    });
  });

  test('pdfSafe keeps latin-1 and maps typography', () {
    expect(pdfSafe('Città · 3×2 …'), 'Città - 3x2 ...');
    expect(pdfSafe('日本'), '??');
  });

  test('settings json roundtrip', () {
    const s = Settings(maxMegapixels: 12, elaQuality: 80, disabledModules: {'ela'});
    final r = Settings.fromJson(s.toJson());
    expect(r.maxMegapixels, 12);
    expect(r.elaQuality, 80);
    expect(r.disabledModules, {'ela'});
  });

  group('rust bridge (needs cargo build -p forensica-bridge)', () {
    final lib = _bridgeLib();
    setUpAll(() async {
      if (lib != null) await RustLib.init(externalLibrary: ExternalLibrary.open(lib));
    });

    test('opens a file, analyses it and builds a PDF', () async {
      final dir = await Directory.systemTemp.createTemp('forensica_test');
      try {
        // Copy the C2PA fixture so the original stays untouched.
        final src = File('../testdata/c2pa/CA.jpg');
        final f = await src.copy('${dir.path}/CA.jpg');
        final before = await f.readAsBytes();
        final s = await engine.openSession(path: f.path);
        expect(s.sha256.length, 64);
        expect(s.format, 'Jpeg');
        ReportDto? report;
        await for (final e in engine.analyze(
          id: s.id,
          options: const AnalysisOptions(maxPixels: 0, elaQuality: 90, modules: [], extra: []),
        )) {
          if (e.report != null) report = e.report;
        }
        expect(report, isNotNull);
        final codes = [for (final m in report!.modules) ...m.findings.map((f) => f.code)];
        expect(codes, contains('c2pa.valid'));
        final pdf = await buildPdfReport(session: s, report: report, l: AppLocalizationsEn());
        expect(pdf.sublist(0, 4), [0x25, 0x50, 0x44, 0x46]); // %PDF
        final ela = await engine.renderEla(id: s.id, quality: 80);
        expect(ela.png.sublist(1, 4), [0x50, 0x4E, 0x47]);
        engine.closeSession(id: s.id);
        expect(await f.readAsBytes(), before, reason: 'original must never be modified');
      } finally {
        await dir.delete(recursive: true);
      }
    }, skip: lib == null ? 'bridge library not built' : false);
  });
}
