import 'dart:io';

import 'package:desktop_drop/desktop_drop.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

import '../../l10n/app_localizations.dart';
import '../state/analysis.dart';

/// Landing screen: open or drop a file.
class HomeScreen extends ConsumerStatefulWidget {
  const HomeScreen({super.key});

  @override
  ConsumerState<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends ConsumerState<HomeScreen> {
  bool _dragging = false;

  Future<void> _open(String path) async {
    final router = GoRouter.of(context);
    final messenger = ScaffoldMessenger.of(context);
    final l = AppLocalizations.of(context);
    await ref.read(analysisProvider.notifier).open(path);
    final err = ref.read(analysisProvider).error;
    if (err != null && ref.read(analysisProvider).session == null) {
      messenger.showSnackBar(SnackBar(content: Text(l.openFailed(err))));
      return;
    }
    router.go('/analysis');
  }

  Future<void> _pick() async {
    final f = await FilePicker.pickFile(
      type: Platform.isAndroid ? FileType.image : FileType.any,
    );
    final path = f?.path;
    if (path != null) await _open(path);
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final opening = ref.watch(analysisProvider.select((s) => s.opening));
    Widget body = Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 560),
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                Icons.image_search,
                size: 72,
                color: theme.colorScheme.primary,
                semanticLabel: l.appTitle,
              ),
              const SizedBox(height: 16),
              Text(l.appTitle, style: theme.textTheme.headlineMedium),
              const SizedBox(height: 8),
              Text(
                l.tagline,
                style: theme.textTheme.titleMedium,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 32),
              AnimatedContainer(
                duration: const Duration(milliseconds: 150),
                padding: const EdgeInsets.all(32),
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(16),
                  border: Border.all(
                    color: _dragging
                        ? theme.colorScheme.primary
                        : theme.colorScheme.outlineVariant,
                    width: _dragging ? 3 : 1.5,
                  ),
                  color: _dragging
                      ? theme.colorScheme.primaryContainer.withValues(
                          alpha: 0.3,
                        )
                      : null,
                ),
                child: Column(
                  children: [
                    if (opening)
                      const CircularProgressIndicator()
                    else
                      FilledButton.icon(
                        onPressed: _pick,
                        icon: const Icon(Icons.folder_open),
                        label: Text(l.openImage),
                      ),
                    if (!Platform.isAndroid) ...[
                      const SizedBox(height: 12),
                      Text(l.dropHere, style: theme.textTheme.bodyMedium),
                    ],
                  ],
                ),
              ),
              const SizedBox(height: 24),
              Row(
                children: [
                  Icon(
                    Icons.lock_outline,
                    size: 18,
                    color: theme.colorScheme.secondary,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      l.privacyNote,
                      style: theme.textTheme.bodySmall,
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
    if (!Platform.isAndroid) {
      body = DropTarget(
        onDragEntered: (_) => setState(() => _dragging = true),
        onDragExited: (_) => setState(() => _dragging = false),
        onDragDone: (d) {
          setState(() => _dragging = false);
          if (d.files.isNotEmpty) _open(d.files.first.path);
        },
        child: body,
      );
    }
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.keyO, control: true): _pick,
      },
      child: Focus(
        autofocus: true,
        child: Scaffold(
          appBar: AppBar(
            actions: [
              if (Platform.isLinux)
                IconButton(
                  tooltip: l.batch,
                  icon: const Icon(Icons.dynamic_feed_outlined),
                  onPressed: () => context.push('/batch'),
                ),
              IconButton(
                tooltip: l.settings,
                icon: const Icon(Icons.settings_outlined),
                onPressed: () => context.push('/settings'),
              ),
            ],
          ),
          body: body,
        ),
      ),
    );
  }
}
