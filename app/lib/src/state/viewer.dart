import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../rust/api/dto.dart';

/// How the selected layer is compared with the original.
enum CompareMode { overlay, sideBySide, swipe }

/// Viewer UI state shared by the viewer and the side panels.
@immutable
class ViewerState {
  const ViewerState({
    this.layerId,
    this.opacity = 0.7,
    this.mode = CompareMode.overlay,
    this.swipe = 0.5,
    this.inspector = false,
    this.pixel,
  });

  /// Selected overlay layer id (null = original only).
  final String? layerId;
  final double opacity;
  final CompareMode mode;

  /// Swipe divider position 0..1.
  final double swipe;
  final bool inspector;
  final PixelDto? pixel;

  ViewerState copyWith({
    String? Function()? layerId,
    double? opacity,
    CompareMode? mode,
    double? swipe,
    bool? inspector,
    PixelDto? Function()? pixel,
  }) => ViewerState(
    layerId: layerId != null ? layerId() : this.layerId,
    opacity: opacity ?? this.opacity,
    mode: mode ?? this.mode,
    swipe: swipe ?? this.swipe,
    inspector: inspector ?? this.inspector,
    pixel: pixel != null ? pixel() : this.pixel,
  );
}

class ViewerNotifier extends Notifier<ViewerState> {
  @override
  ViewerState build() => const ViewerState();

  void select(String? id) => state = state.copyWith(layerId: () => id);
  void setOpacity(double v) => state = state.copyWith(opacity: v);
  void setMode(CompareMode m) => state = state.copyWith(mode: m);
  void setSwipe(double v) => state = state.copyWith(swipe: v.clamp(0, 1));
  void toggleInspector() =>
      state = state.copyWith(inspector: !state.inspector, pixel: () => null);
  void setPixel(PixelDto? p) => state = state.copyWith(pixel: () => p);
  void reset() => state = const ViewerState();
}

final viewerProvider = NotifierProvider<ViewerNotifier, ViewerState>(
  ViewerNotifier.new,
);
