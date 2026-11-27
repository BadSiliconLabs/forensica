import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../l10n/app_localizations.dart';
import '../rust/api/dto.dart';
import '../rust/api/engine.dart' as engine;
import '../state/analysis.dart';

Future<List<String>> _pickImages() async {
  final files = await FilePicker.pickFiles(type: FileType.image);
  return [for (final f in files) if (f.path != null) f.path!];
}

/// PRNU comparison against user-chosen reference photos.
class PrnuSection extends ConsumerStatefulWidget {
  const PrnuSection({super.key});

  @override
  ConsumerState<PrnuSection> createState() => _PrnuSectionState();
}

class _PrnuSectionState extends ConsumerState<PrnuSection> {
  PrnuDto? _result;
  String? _error;
  bool _busy = false;

  Future<void> _run() async {
    final s = ref.read(analysisProvider).session;
    if (s == null) return;
    final refs = await _pickImages();
    if (refs.isEmpty) return;
    setState(() {
      _busy = true;
      _error = null;
      _result = null;
    });
    try {
      final r = await engine.prnuCompare(id: s.id, references: refs);
      setState(() => _result = r);
    } catch (e) {
      setState(() => _error = '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final r = _result;
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(l.prnuHint, style: theme.textTheme.bodySmall),
          const SizedBox(height: 8),
          FilledButton.tonalIcon(
            onPressed: _busy ? null : _run,
            icon: const Icon(Icons.photo_library_outlined),
            label: Text(l.chooseReferences),
          ),
          if (_busy) const Padding(padding: EdgeInsets.only(top: 8), child: LinearProgressIndicator()),
          if (_error != null) Padding(padding: const EdgeInsets.only(top: 8), child: Text(_error!, style: TextStyle(color: theme.colorScheme.error))),
          if (r != null)
            Card(
              margin: const EdgeInsets.only(top: 8),
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      r.pce >= r.threshold
                          ? l.prnuMatch(r.pce.toStringAsFixed(1), r.threshold.toStringAsFixed(0))
                          : l.prnuNoMatch(r.pce.toStringAsFixed(1), r.threshold.toStringAsFixed(0)),
                      style: theme.textTheme.bodyMedium,
                    ),
                    const SizedBox(height: 4),
                    Text(l.prnuDetails(r.references, r.skipped, r.crop), style: theme.textTheme.bodySmall),
                  ],
                ),
              ),
            ),
        ],
      ),
    );
  }
}

/// Near-duplicate search by perceptual hash.
class DuplicatesSection extends ConsumerStatefulWidget {
  const DuplicatesSection({super.key});

  @override
  ConsumerState<DuplicatesSection> createState() => _DuplicatesSectionState();
}

class _DuplicatesSectionState extends ConsumerState<DuplicatesSection> {
  List<SimilarDto>? _results;
  bool _busy = false;

  Future<void> _search(Future<List<String>> Function() source) async {
    final s = ref.read(analysisProvider).session;
    if (s == null) return;
    final paths = await source();
    if (paths.isEmpty) return;
    setState(() {
      _busy = true;
      _results = null;
    });
    try {
      final r = await engine.findSimilar(id: s.id, paths: paths, maxDistance: 12);
      setState(() => _results = r);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<List<String>> _folder() async {
    // ignore: deprecated_member_use
    final d = await FilePicker.getDirectoryPath();
    if (d == null) return const [];
    return engine.listImages(dir: d);
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final r = _results;
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(l.duplicatesHint, style: theme.textTheme.bodySmall),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            children: [
              if (!Platform.isAndroid)
                FilledButton.tonalIcon(
                  onPressed: _busy ? null : () => _search(_folder),
                  icon: const Icon(Icons.folder_open),
                  label: Text(l.chooseFolder),
                ),
              FilledButton.tonalIcon(
                onPressed: _busy ? null : () => _search(_pickImages),
                icon: const Icon(Icons.photo_library_outlined),
                label: Text(l.chooseImages),
              ),
            ],
          ),
          if (_busy) Padding(padding: const EdgeInsets.only(top: 8), child: Row(children: [const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2)), const SizedBox(width: 8), Text(l.searching)])),
          if (r != null && r.isEmpty) Padding(padding: const EdgeInsets.only(top: 8), child: Text(l.noDuplicates)),
          if (r != null)
            for (final m in r)
              ListTile(
                dense: true,
                contentPadding: EdgeInsets.zero,
                leading: ClipRRect(
                  borderRadius: BorderRadius.circular(4),
                  child: Image.file(File(m.path), width: 48, height: 48, fit: BoxFit.cover, cacheWidth: 96, errorBuilder: (_, _, _) => const Icon(Icons.image_not_supported_outlined)),
                ),
                title: Text(m.path.split(Platform.pathSeparator).last, overflow: TextOverflow.ellipsis),
                subtitle: Text(l.similarItem(m.phashDistance, m.width, m.height)),
                trailing: IconButton(
                  tooltip: l.openThis,
                  icon: const Icon(Icons.open_in_new),
                  onPressed: () => ref.read(analysisProvider.notifier).open(m.path),
                ),
              ),
        ],
      ),
    );
  }
}
