import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

import '../../l10n/app_localizations.dart';
import '../report/export.dart';
import '../state/analysis.dart';
import '../state/viewer.dart';
import '../util/format.dart';
import '../widgets/findings.dart';
import '../widgets/metadata_view.dart';
import '../widgets/tools_panel.dart';
import '../widgets/viewer.dart';

/// Main analysis screen. Wide: viewer + side panel. Narrow: tabs.
class AnalysisScreen extends ConsumerWidget {
  const AnalysisScreen({super.key});

  Future<void> _export(BuildContext context, WidgetRef ref, bool pdf) async {
    final l = AppLocalizations.of(context);
    final messenger = ScaffoldMessenger.of(context);
    try {
      final where = pdf
          ? await exportPdf(ref.read(analysisProvider), l)
          : await exportJson(ref.read(analysisProvider));
      if (where != null) {
        messenger.showSnackBar(SnackBar(content: Text(l.saved(where))));
      }
    } catch (e) {
      messenger.showSnackBar(SnackBar(content: Text(l.exportFailed('$e'))));
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final a = ref.watch(analysisProvider);
    final s = a.session;
    if (s == null) {
      return Scaffold(
        appBar: AppBar(),
        body: Center(
          child: a.opening
              ? const CircularProgressIndicator()
              : Text(a.error ?? ''),
        ),
      );
    }
    final notifier = ref.read(analysisProvider.notifier);
    Future<void> closeFile() async {
      ref.read(viewerProvider.notifier).reset();
      await notifier.close();
      if (context.mounted) context.go('/');
    }

    final appBar = AppBar(
      leading: IconButton(
        tooltip: l.closeFile,
        icon: const Icon(Icons.close),
        onPressed: () => closeFile(),
      ),
      title: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(s.name, overflow: TextOverflow.ellipsis),
          InkWell(
            onTap: () {
              Clipboard.setData(ClipboardData(text: s.sha256));
              ScaffoldMessenger.of(context)
                  .showSnackBar(SnackBar(content: Text(l.copied)));
            },
            child: Text(
              '${l.sha256} ${shortSha(s.sha256)}',
              style: Theme.of(context).textTheme.labelSmall
                  ?.copyWith(fontFamily: 'monospace'),
            ),
          ),
        ],
      ),
      actions: [
        if (a.running)
          TextButton.icon(
            onPressed: notifier.cancel,
            icon: const Icon(Icons.stop_circle_outlined),
            label: Text(l.cancel),
          )
        else
          IconButton(
            tooltip: l.rerun,
            icon: const Icon(Icons.refresh),
            onPressed: notifier.run,
          ),
        PopupMenuButton<String>(
          tooltip: l.exportReport,
          enabled: a.report != null && !a.running,
          onSelected: (v) => _export(context, ref, v == 'pdf'),
          itemBuilder: (c) => [
            PopupMenuItem(
              value: 'pdf',
              child: ListTile(
                leading: const Icon(Icons.picture_as_pdf_outlined),
                title: Text(l.exportPdf),
              ),
            ),
            PopupMenuItem(
              value: 'json',
              child: ListTile(
                leading: const Icon(Icons.data_object),
                title: Text(l.exportJson),
              ),
            ),
          ],
          icon: const Icon(Icons.ios_share),
        ),
        IconButton(
          tooltip: l.settings,
          icon: const Icon(Icons.settings_outlined),
          onPressed: () => context.push('/settings'),
        ),
      ],
      bottom: a.running
          ? PreferredSize(
              preferredSize: const Size.fromHeight(4),
              child: Semantics(
                label: l.analyzing(a.completed, a.total),
                child: LinearProgressIndicator(
                  value: a.total == 0 ? null : a.completed / a.total,
                ),
              ),
            )
          : null,
    );

    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.keyR, control: true): () {
          if (!a.running) notifier.run();
        },
        const SingleActivator(LogicalKeyboardKey.keyW, control: true):
            closeFile,
        const SingleActivator(LogicalKeyboardKey.escape): () {
          if (a.running) notifier.cancel();
        },
      },
      child: Focus(
        autofocus: true,
        child: LayoutBuilder(
          builder: (context, cons) {
            // Complete mode: landscape (phones/tablets) or any wide window (desktop).
            // Simplified mode: portrait phones — findings, image + quick layers, metadata.
            final landscape =
                cons.maxWidth > cons.maxHeight && cons.maxWidth >= 600;
            final wide = cons.maxWidth >= 900 || landscape;
            if (wide) {
              return Scaffold(
                appBar: appBar,
                body: Row(
                  children: [
                    const Expanded(child: ImageViewer()),
                    const VerticalDivider(width: 1),
                    SizedBox(
                      width: cons.maxWidth >= 1300
                          ? 480
                          : (cons.maxWidth * 0.45).clamp(300.0, 400.0),
                      child: DefaultTabController(
                        length: 3,
                        child: Column(
                          children: [
                            TabBar(
                              tabs: [
                                Tab(text: l.findings),
                                Tab(text: l.metadata),
                                Tab(text: l.tools),
                              ],
                            ),
                            const Expanded(
                              child: TabBarView(
                                children: [
                                  FindingsPanel(),
                                  MetadataPanel(),
                                  ToolsPanel(),
                                ],
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ],
                ),
              );
            }
            return DefaultTabController(
              length: 3,
              child: Scaffold(
                appBar: appBar,
                body: Column(
                  children: [
                    TabBar(
                      isScrollable: true,
                      tabs: [
                        Tab(text: l.findings),
                        Tab(text: l.image),
                        Tab(text: l.metadata),
                      ],
                    ),
                    Expanded(
                      child: TabBarView(
                        physics: const NeverScrollableScrollPhysics(),
                        children: [
                          const FindingsPanel(),
                          Column(
                            children: [
                              const Expanded(child: ImageViewer()),
                              const _QuickLayerBar(),
                            ],
                          ),
                          const MetadataPanel(),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            );
          },
        ),
      ),
    );
  }
}

/// Simplified (portrait) mode: horizontal chips to switch layers, plus a hint
/// that landscape opens the complete mode.
class _QuickLayerBar extends ConsumerWidget {
  const _QuickLayerBar();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final a = ref.watch(analysisProvider);
    final v = ref.watch(viewerProvider);
    final layers = a.layers.where((x) => x.kind != 'standalone').toList();
    return Material(
      elevation: 2,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          SizedBox(
            height: 52,
            child: ListView(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
              children: [
                ChoiceChip(
                  label: Text(l.originalImage),
                  selected: v.layerId == null,
                  onSelected: (_) =>
                      ref.read(viewerProvider.notifier).select(null),
                ),
                for (final x in layers)
                  Padding(
                    padding: const EdgeInsets.only(left: 6),
                    child: ChoiceChip(
                      label: Text(x.title),
                      selected: v.layerId == x.id,
                      onSelected: (_) =>
                          ref.read(viewerProvider.notifier).select(x.id),
                    ),
                  ),
              ],
            ),
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(12, 0, 12, 8),
            child: Row(
              children: [
                const Icon(Icons.screen_rotation, size: 16),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    l.rotateHint,
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
