import 'dart:ui' as ui;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../l10n/app_localizations.dart';
import '../rust/api/dto.dart';
import '../rust/api/engine.dart' as engine;
import '../state/analysis.dart';
import '../state/viewer.dart';

/// Zoomable image viewer with overlay, side-by-side and swipe comparison.
class ImageViewer extends ConsumerStatefulWidget {
  const ImageViewer({super.key});

  @override
  ConsumerState<ImageViewer> createState() => _ImageViewerState();
}

class _ImageViewerState extends ConsumerState<ImageViewer> {
  final _controller = TransformationController();
  final _controller2 = TransformationController();
  bool _syncing = false;

  @override
  void initState() {
    super.initState();
    _controller.addListener(() => _sync(_controller, _controller2));
    _controller2.addListener(() => _sync(_controller2, _controller));
  }

  void _sync(TransformationController from, TransformationController to) {
    if (_syncing) return;
    _syncing = true;
    to.value = from.value.clone();
    _syncing = false;
  }

  @override
  void dispose() {
    _controller.dispose();
    _controller2.dispose();
    super.dispose();
  }

  Future<void> _inspect(Offset local, Size box, SessionInfo s) async {
    final x = (local.dx / box.width * s.width).floor().clamp(0, s.width - 1);
    final y = (local.dy / box.height * s.height).floor().clamp(0, s.height - 1);
    try {
      final p = await engine.pixelAt(id: s.id, x: x, y: y);
      ref.read(viewerProvider.notifier).setPixel(p);
    } catch (_) {
      // Out of range or session closed.
    }
  }

  @override
  Widget build(BuildContext context) {
    final a = ref.watch(analysisProvider);
    final v = ref.watch(viewerProvider);
    final l = AppLocalizations.of(context);
    final s = a.session;
    final img = a.preview;
    if (s == null || img == null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Text(l.notDecodable, textAlign: TextAlign.center),
        ),
      );
    }
    final layers = a.layers.where((x) => x.kind != 'standalone');
    VizDto? layer;
    for (final x in layers) {
      if (x.id == v.layerId) layer = x;
    }
    final aspect = s.width / (s.height == 0 ? 1 : s.height);

    Widget frame(TransformationController c, Widget Function(Size) content) => LayoutBuilder(
          builder: (context, cons) {
            var w = cons.maxWidth;
            var h = w / aspect;
            if (h > cons.maxHeight) {
              h = cons.maxHeight;
              w = h * aspect;
            }
            final box = Size(w, h);
            return InteractiveViewer(
              transformationController: c,
              maxScale: 64,
              minScale: 0.5,
              child: Center(
                child: SizedBox.fromSize(
                  size: box,
                  child: GestureDetector(
                    onTapUp: v.inspector ? (d) => _inspect(d.localPosition, box, s) : null,
                    child: content(box),
                  ),
                ),
              ),
            );
          },
        );

    Widget base() => RawImage(image: img, fit: BoxFit.fill, filterQuality: FilterQuality.none);
    Widget overlay(VizDto x) => Stack(
          fit: StackFit.expand,
          children: [
            Image.memory(x.png, fit: BoxFit.fill, filterQuality: FilterQuality.none, gaplessPlayback: true),
            if (x.lines.isNotEmpty) CustomPaint(painter: _LinesPainter(x.lines, s.width, s.height)),
          ],
        );
    Widget marker(Size box) {
      final p = v.pixel;
      if (!v.inspector || p == null) return const SizedBox.shrink();
      final dx = (p.x + 0.5) / s.width * box.width;
      final dy = (p.y + 0.5) / s.height * box.height;
      return Positioned(
        left: dx - 6,
        top: dy - 6,
        child: IgnorePointer(
          child: Container(
            width: 12,
            height: 12,
            decoration: BoxDecoration(border: Border.all(color: Colors.cyanAccent, width: 2), shape: BoxShape.circle),
          ),
        ),
      );
    }

    Widget view;
    if (layer == null) {
      view = frame(_controller, (box) => Stack(fit: StackFit.expand, children: [base(), marker(box)]));
    } else {
      final lay = layer;
      switch (v.mode) {
        case CompareMode.overlay:
          view = frame(
            _controller,
            (box) => Stack(
              fit: StackFit.expand,
              children: [base(), Opacity(opacity: v.opacity, child: overlay(lay)), marker(box)],
            ),
          );
        case CompareMode.sideBySide:
          view = Row(
            children: [
              Expanded(child: frame(_controller, (box) => Stack(fit: StackFit.expand, children: [base(), marker(box)]))),
              const VerticalDivider(width: 2),
              Expanded(child: frame(_controller2, (box) => Stack(fit: StackFit.expand, children: [overlay(lay), marker(box)]))),
            ],
          );
        case CompareMode.swipe:
          view = Stack(
            children: [
              frame(
                _controller,
                (box) => Stack(
                  fit: StackFit.expand,
                  children: [
                    base(),
                    ClipRect(clipper: _SwipeClipper(v.swipe), child: overlay(lay)),
                    marker(box),
                  ],
                ),
              ),
              Positioned(
                left: 16,
                right: 16,
                bottom: 8,
                child: Semantics(
                  label: l.modeSwipe,
                  child: Slider(value: v.swipe, onChanged: ref.read(viewerProvider.notifier).setSwipe),
                ),
              ),
            ],
          );
      }
    }
    return ColoredBox(
      color: Theme.of(context).colorScheme.surfaceContainerLowest,
      child: Stack(
        children: [
          Positioned.fill(child: view),
          if (v.inspector) Positioned(top: 8, left: 8, child: _PixelCard(pixel: v.pixel)),
          if (layer != null && layer.legendLow.isNotEmpty)
            Positioned(right: 8, top: 8, child: _Legend(viz: layer)),
        ],
      ),
    );
  }
}

class _SwipeClipper extends CustomClipper<Rect> {
  _SwipeClipper(this.t);
  final double t;
  @override
  Rect getClip(Size size) => Rect.fromLTWH(size.width * t, 0, size.width * (1 - t), size.height);
  @override
  bool shouldReclip(_SwipeClipper old) => old.t != t;
}

class _LinesPainter extends CustomPainter {
  _LinesPainter(this.lines, this.w, this.h);
  final List<LineDto> lines;
  final int w;
  final int h;

  @override
  void paint(Canvas canvas, Size size) {
    final sx = size.width / w;
    final sy = size.height / h;
    final p = Paint()
      ..color = Colors.cyanAccent
      ..strokeWidth = 1.2
      ..style = PaintingStyle.stroke;
    final dot = Paint()..color = Colors.yellowAccent;
    for (final l in lines) {
      final a = Offset(l.x1 * sx, l.y1 * sy);
      final b = Offset(l.x2 * sx, l.y2 * sy);
      canvas.drawLine(a, b, p);
      canvas.drawCircle(a, 2, dot);
      canvas.drawCircle(b, 2, dot);
    }
  }

  @override
  bool shouldRepaint(_LinesPainter old) => old.lines != lines;
}

class _PixelCard extends StatelessWidget {
  const _PixelCard({required this.pixel});
  final PixelDto? pixel;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final p = pixel;
    final t = Theme.of(context).textTheme.bodySmall;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(8),
        child: p == null
            ? Text(l.tapToInspect, style: t)
            : Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Container(width: 28, height: 28, color: Color.fromARGB(255, p.r, p.g, p.b)),
                  const SizedBox(width: 8),
                  Text(
                    'x ${p.x}  y ${p.y}\nR ${p.r}  G ${p.g}  B ${p.b}\nY ${p.luma.toStringAsFixed(1)}  8×8 μ ${p.blockMean.toStringAsFixed(1)}',
                    style: t?.copyWith(fontFeatures: const [ui.FontFeature.tabularFigures()]),
                  ),
                ],
              ),
      ),
    );
  }
}

class _Legend extends StatelessWidget {
  const _Legend({required this.viz});
  final VizDto viz;

  static List<Color> _colors(String cmap) => switch (cmap) {
        'gray' => const [Colors.black, Colors.white],
        'diverging' => const [Color(0xFF2166AC), Color(0xFFF7F7F7), Color(0xFFB2182B)],
        _ => const [Color(0xFF000004), Color(0xFF55106D), Color(0xFFBA3655), Color(0xFFF98C0A), Color(0xFFFCFFA4)],
      };

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).textTheme.labelSmall;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 140,
              height: 10,
              decoration: BoxDecoration(gradient: LinearGradient(colors: _colors(viz.colormap))),
            ),
            const SizedBox(height: 4),
            SizedBox(
              width: 140,
              child: Row(
                children: [
                  Expanded(child: Text(viz.legendLow, style: t)),
                  Text(viz.legendHigh, style: t),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
