import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../l10n/app_localizations.dart';
import '../rust/api/dto.dart';
import '../rust/api/engine.dart' as engine;
import '../state/analysis.dart';
import '../state/viewer.dart';
import '../util/format.dart';
import 'source_tools.dart';

/// Layers, comparison controls, interactive renderers and file details.
class ToolsPanel extends ConsumerStatefulWidget {
  const ToolsPanel({super.key});

  @override
  ConsumerState<ToolsPanel> createState() => _ToolsPanelState();
}

class _ToolsPanelState extends ConsumerState<ToolsPanel> {
  double _ela = 90;
  String _channel = 'luma';
  double _bit = 0;
  bool _busy = false;

  Future<void> _render(Future<VizDto> Function() f) async {
    setState(() => _busy = true);
    try {
      final v = await f();
      ref.read(analysisProvider.notifier).addLayer(v);
      ref.read(viewerProvider.notifier).select(v.id);
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context)
            .showSnackBar(SnackBar(content: Text('$e')));
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final a = ref.watch(analysisProvider);
    final v = ref.watch(viewerProvider);
    final vn = ref.read(viewerProvider.notifier);
    final s = a.session;
    final theme = Theme.of(context);
    final aligned = a.layers.where((x) => x.kind != 'standalone').toList();
    final standalone = a.layers.where((x) => x.kind == 'standalone').toList();
    Widget header(String t) => Padding(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 4),
      child: Semantics(
        header: true,
        child: Text(
          t,
          style: theme.textTheme.titleSmall?.copyWith(
            color: theme.colorScheme.primary,
          ),
        ),
      ),
    );
    return ListView(
      children: [
        header(l.layers),
        RadioGroup<String>(
          groupValue: v.layerId ?? '',
          onChanged: (id) => vn.select((id == null || id.isEmpty) ? null : id),
          child: Column(
            children: [
              RadioListTile<String>(
                value: '',
                title: Text(l.originalImage),
                dense: true,
              ),
              for (final x in aligned)
                RadioListTile<String>(
                  value: x.id,
                  dense: true,
                  title: Text(x.title),
                  subtitle: v.layerId == x.id
                      ? Text(x.description, style: theme.textTheme.bodySmall)
                      : null,
                ),
            ],
          ),
        ),
        if (aligned.isEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Text(l.selectLayerHint),
          ),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: SegmentedButton<CompareMode>(
            segments: [
              ButtonSegment(
                value: CompareMode.overlay,
                label: Text(l.modeOverlay),
                icon: const Icon(Icons.layers_outlined),
              ),
              ButtonSegment(
                value: CompareMode.sideBySide,
                label: Text(l.modeSideBySide),
                icon: const Icon(Icons.view_column_outlined),
              ),
              ButtonSegment(
                value: CompareMode.swipe,
                label: Text(l.modeSwipe),
                icon: const Icon(Icons.compare),
              ),
            ],
            selected: {v.mode},
            onSelectionChanged: (m) => vn.setMode(m.first),
          ),
        ),
        if (v.mode == CompareMode.overlay)
          ListTile(
            title: Text(l.overlayOpacity),
            subtitle: Slider(
              value: v.opacity,
              onChanged: vn.setOpacity,
              label: '${(v.opacity * 100).round()}%',
            ),
          ),
        SwitchListTile(
          title: Text(l.pixelInspector),
          secondary: const Icon(Icons.colorize_outlined),
          value: v.inspector,
          onChanged: (_) => vn.toggleInspector(),
        ),
        if (s != null && s.decodable) ...[
          header(l.tools),
          ListTile(
            title: Text(l.elaQuality(_ela.round())),
            subtitle: Slider(
              min: 50,
              max: 99,
              divisions: 49,
              value: _ela,
              label: '${_ela.round()}',
              onChanged: (x) => setState(() => _ela = x),
            ),
            trailing: FilledButton.tonal(
              onPressed: _busy
                  ? null
                  : () => _render(
                      () => engine.renderEla(id: s.id, quality: _ela.round()),
                    ),
              child: Text(l.render),
            ),
          ),
          ListTile(
            title: Text(l.channel),
            trailing: DropdownButton<String>(
              value: _channel,
              items: const [
                DropdownMenuItem(value: 'luma', child: Text('Luma (Y)')),
                DropdownMenuItem(value: 'red', child: Text('Red')),
                DropdownMenuItem(value: 'green', child: Text('Green')),
                DropdownMenuItem(value: 'blue', child: Text('Blue')),
                DropdownMenuItem(value: 'cb', child: Text('Chroma Cb')),
                DropdownMenuItem(value: 'cr', child: Text('Chroma Cr')),
                DropdownMenuItem(value: 'hue', child: Text('Hue')),
                DropdownMenuItem(
                  value: 'saturation',
                  child: Text('Saturation'),
                ),
              ],
              onChanged: (c) => setState(() => _channel = c ?? 'luma'),
            ),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Wrap(
              spacing: 8,
              children: [
                FilledButton.tonal(
                  onPressed: _busy
                      ? null
                      : () => _render(
                          () =>
                              engine.renderChannel(id: s.id, channel: _channel),
                        ),
                  child: Text(l.render),
                ),
              ],
            ),
          ),
          ListTile(
            title: Text(l.bitPlane(_bit.round())),
            subtitle: Slider(
              min: 0,
              max: 7,
              divisions: 7,
              value: _bit,
              label: '${_bit.round()}',
              onChanged: (x) => setState(() => _bit = x),
            ),
            trailing: FilledButton.tonal(
              onPressed: _busy
                  ? null
                  : () => _render(
                      () => engine.renderBitplane(
                        id: s.id,
                        channel: _channel,
                        bit: _bit.round(),
                      ),
                    ),
              child: Text(l.render),
            ),
          ),
          if (_busy) const LinearProgressIndicator(),
          header(l.duplicates),
          const DuplicatesSection(),
          header(l.prnu),
          const PrnuSection(),
        ],
        if (standalone.isNotEmpty) ...[
          header(l.otherViews),
          for (final x in standalone)
            ListTile(
              leading: ClipRRect(
                borderRadius: BorderRadius.circular(4),
                child: Image.memory(
                  x.png,
                  width: 56,
                  height: 56,
                  fit: BoxFit.cover,
                ),
              ),
              title: Text(x.title),
              subtitle: Text(
                x.description,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
              onTap: () => showDialog<void>(
                context: context,
                builder: (c) => Dialog(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Padding(
                        padding: const EdgeInsets.all(12),
                        child: Text(
                          x.title,
                          style: theme.textTheme.titleMedium,
                        ),
                      ),
                      Flexible(
                        child: InteractiveViewer(
                          maxScale: 16,
                          child: Image.memory(
                            x.png,
                            filterQuality: FilterQuality.none,
                          ),
                        ),
                      ),
                      Padding(
                        padding: const EdgeInsets.all(12),
                        child: Text(x.description),
                      ),
                    ],
                  ),
                ),
              ),
            ),
        ],
        if (s != null) ...[
          header(l.fileDetails),
          ListTile(dense: true, title: Text(s.name), subtitle: Text(s.path)),
          ListTile(
            dense: true,
            title: Text(l.sha256),
            subtitle: SelectableText(
              s.sha256,
              style: const TextStyle(fontFamily: 'monospace'),
            ),
            trailing: IconButton(
              tooltip: l.copySha,
              icon: const Icon(Icons.copy),
              onPressed: () {
                Clipboard.setData(ClipboardData(text: s.sha256));
                ScaffoldMessenger.of(context)
                    .showSnackBar(SnackBar(content: Text(l.copied)));
              },
            ),
          ),
          ListTile(
            dense: true,
            title: Text(l.format),
            trailing: Text('${s.format} (${s.mime})'),
          ),
          ListTile(
            dense: true,
            title: Text(l.size),
            trailing: Text(formatBytes(s.size)),
          ),
          if (s.width > 0)
            ListTile(
              dense: true,
              title: Text(l.dimensions),
              trailing: Text('${s.width} × ${s.height}'),
            ),
          if (s.extensionMismatch)
            ListTile(
              dense: true,
              leading: const Icon(Icons.warning_amber),
              title: Text(l.extensionMismatch),
            ),
          if (a.report != null) ...[
            header(l.moduleTimes),
            for (final m in a.report!.modules)
              ListTile(
                dense: true,
                title: Text(m.name),
                trailing: Text(m.cached ? l.cached : '${m.durationMs} ms'),
              ),
          ],
        ],
        const SizedBox(height: 24),
      ],
    );
  }
}
