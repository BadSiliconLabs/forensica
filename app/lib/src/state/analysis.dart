import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../platform/intake.dart';
import '../rust/api/dto.dart';
import '../rust/api/engine.dart' as engine;
import 'settings.dart';
import 'viewer.dart';

/// State of the currently open file and its analysis.
@immutable
class AnalysisState {
  const AnalysisState({
    this.session,
    this.preview,
    this.previewScale = 1,
    this.opening = false,
    this.running = false,
    this.completed = 0,
    this.total = 0,
    this.stages = const {},
    this.report,
    this.error,
    this.customLayers = const [],
  });

  final SessionInfo? session;
  final ui.Image? preview;
  final double previewScale;
  final bool opening;
  final bool running;
  final int completed;
  final int total;
  final Map<String, String> stages;
  final ReportDto? report;
  final String? error;

  /// Layers rendered on demand by the Tools panel (ELA at custom quality, bit planes...).
  final List<VizDto> customLayers;

  AnalysisState copyWith({
    SessionInfo? session,
    ui.Image? preview,
    double? previewScale,
    bool? opening,
    bool? running,
    int? completed,
    int? total,
    Map<String, String>? stages,
    ReportDto? report,
    String? Function()? error,
    List<VizDto>? customLayers,
  }) => AnalysisState(
    session: session ?? this.session,
    preview: preview ?? this.preview,
    previewScale: previewScale ?? this.previewScale,
    opening: opening ?? this.opening,
    running: running ?? this.running,
    completed: completed ?? this.completed,
    total: total ?? this.total,
    stages: stages ?? this.stages,
    report: report ?? this.report,
    error: error != null ? error() : this.error,
    customLayers: customLayers ?? this.customLayers,
  );

  /// Every finding, most severe first.
  List<FindingDto> get findings {
    final r = report;
    if (r == null) return const [];
    final all = [for (final m in r.modules) ...m.findings];
    all.sort((a, b) {
      final s = _rank(b.severity).compareTo(_rank(a.severity));
      return s != 0 ? s : b.confidence.compareTo(a.confidence);
    });
    return all;
  }

  /// All visualizations from the report plus custom layers.
  List<VizDto> get layers => [
    for (final m in report?.modules ?? const <ModuleRunDto>[])
      ...m.visualizations,
    ...customLayers,
  ];

  static int _rank(String s) => switch (s) {
    'high' => 3,
    'medium' => 2,
    'low' => 1,
    _ => 0,
  };
}

Future<ui.Image> _decodeRgba(PreviewDto p) {
  final c = Completer<ui.Image>();
  ui.decodeImageFromPixels(
    p.rgba,
    p.width,
    p.height,
    ui.PixelFormat.rgba8888,
    c.complete,
  );
  return c.future;
}

/// Drives opening, analysing and closing files.
class AnalysisNotifier extends Notifier<AnalysisState> {
  StreamSubscription<AnalysisEvent>? _sub;

  @override
  AnalysisState build() {
    ref.onDispose(() => _sub?.cancel());
    return const AnalysisState();
  }

  /// Opens [path] read-only and starts the analysis.
  Future<void> open(String path) async {
    await close();
    ref.read(viewerProvider.notifier).reset();
    state = const AnalysisState(opening: true);
    try {
      final info = await engine.openSession(path: path);
      ui.Image? img;
      var scale = 1.0;
      if (info.decodable) {
        final p = await engine.preview(
          id: info.id,
          maxSide: Platform.isAndroid ? 2048 : 4096,
        );
        img = await _decodeRgba(p);
        scale = p.scale;
      }
      state = AnalysisState(session: info, preview: img, previewScale: scale);
      await run();
    } catch (e) {
      state = AnalysisState(error: e.toString());
    }
  }

  AnalysisOptions _options() {
    final s = ref.read(settingsProvider);
    final enabled = engine
        .listModules()
        .map((m) => m.id)
        .where((id) => !s.disabledModules.contains(id))
        .toList();
    return AnalysisOptions(
      maxPixels: s.maxMegapixels * 1000000,
      elaQuality: s.elaQuality,
      modules: enabled,
      extra: const [],
    );
  }

  /// (Re-)runs every enabled module.
  Future<void> run() async {
    final s = state.session;
    if (s == null) return;
    await _sub?.cancel();
    state = state.copyWith(
      running: true,
      completed: 0,
      total: 0,
      stages: {},
      error: () => null,
    );
    final done = Completer<void>();
    _sub = engine
        .analyze(id: s.id, options: _options())
        .listen(
          (e) {
            if (e.report != null) {
              state = state.copyWith(
                report: e.report,
                running: false,
                completed: e.completed,
                total: e.total,
              );
            } else {
              state = state.copyWith(
                completed: e.completed,
                total: e.total,
                stages: {...state.stages, e.moduleId: e.stage},
              );
            }
          },
          onError: (Object e) {
            state = state.copyWith(running: false, error: () => e.toString());
            if (!done.isCompleted) done.complete();
          },
          onDone: () {
            if (state.running) state = state.copyWith(running: false);
            if (!done.isCompleted) done.complete();
          },
        );
    return done.future;
  }

  /// Requests cancellation.
  void cancel() {
    final s = state.session;
    if (s != null) engine.cancelAnalysis(id: s.id);
  }

  /// Adds a layer produced by an interactive tool, replacing one with the same id.
  void addLayer(VizDto v) {
    state = state.copyWith(
      customLayers: [...state.customLayers.where((l) => l.id != v.id), v],
    );
  }

  /// Closes the file and deletes temporary copies.
  Future<void> close() async {
    await _sub?.cancel();
    _sub = null;
    final s = state.session;
    if (s != null) {
      engine.cancelAnalysis(id: s.id);
      engine.closeSession(id: s.id);
      await Intake.deleteIfTemporary(s.path);
    }
    state.preview?.dispose();
    state = const AnalysisState();
  }
}

final analysisProvider = NotifierProvider<AnalysisNotifier, AnalysisState>(
  AnalysisNotifier.new,
);
