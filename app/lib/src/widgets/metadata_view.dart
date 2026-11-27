import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../l10n/app_localizations.dart';
import '../state/analysis.dart';

/// A metadata group parsed from module raw JSON.
class MetaGroup {
  MetaGroup(this.name, this.entries);
  final String name;
  final List<(String, String)> entries;
}

/// Extracts `raw.groups` from every module that provides them.
List<MetaGroup> metadataGroups(AnalysisState a) {
  final out = <MetaGroup>[];
  for (final m in a.report?.modules ?? const []) {
    try {
      final raw = jsonDecode(m.rawJson);
      final groups = raw is Map ? raw['groups'] : null;
      if (groups is! List) continue;
      for (final g in groups) {
        if (g is! Map) continue;
        final entries = <(String, String)>[
          for (final e in (g['entries'] as List? ?? const []))
            if (e is Map) ('${e['key']}', '${e['value']}'),
        ];
        out.add(MetaGroup('${g['name']}', entries));
      }
    } catch (_) {
      // Ignore malformed module output.
    }
  }
  return out;
}

/// Searchable, grouped metadata table.
class MetadataPanel extends ConsumerStatefulWidget {
  const MetadataPanel({super.key});

  @override
  ConsumerState<MetadataPanel> createState() => _MetadataPanelState();
}

class _MetadataPanelState extends ConsumerState<MetadataPanel> {
  String _query = '';

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final a = ref.watch(analysisProvider);
    final q = _query.toLowerCase();
    final groups = metadataGroups(a)
        .map((g) => MetaGroup(
              g.name,
              q.isEmpty
                  ? g.entries
                  : g.entries.where((e) => e.$1.toLowerCase().contains(q) || e.$2.toLowerCase().contains(q)).toList(),
            ))
        .where((g) => g.entries.isNotEmpty)
        .toList();
    final theme = Theme.of(context);
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(12),
          child: TextField(
            decoration: InputDecoration(
              prefixIcon: const Icon(Icons.search),
              hintText: l.searchMetadata,
              border: const OutlineInputBorder(),
              isDense: true,
            ),
            onChanged: (v) => setState(() => _query = v),
          ),
        ),
        Expanded(
          child: groups.isEmpty
              ? Center(child: Text(l.noMetadata))
              : ListView.builder(
                  itemCount: groups.length,
                  itemBuilder: (context, i) {
                    final g = groups[i];
                    return ExpansionTile(
                      initiallyExpanded: i < 3 || q.isNotEmpty,
                      title: Text(g.name),
                      subtitle: Text('${g.entries.length}'),
                      children: [
                        for (final (k, v) in g.entries)
                          InkWell(
                            onLongPress: () {
                              Clipboard.setData(ClipboardData(text: '$k: $v'));
                              ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(l.copied)));
                            },
                            child: Padding(
                              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
                              child: Row(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  SizedBox(
                                    width: 160,
                                    child: Text(k, style: theme.textTheme.bodySmall?.copyWith(fontWeight: FontWeight.w600)),
                                  ),
                                  const SizedBox(width: 8),
                                  Expanded(child: SelectableText(v, style: theme.textTheme.bodySmall)),
                                ],
                              ),
                            ),
                          ),
                        const SizedBox(height: 8),
                      ],
                    );
                  },
                ),
        ),
      ],
    );
  }
}
