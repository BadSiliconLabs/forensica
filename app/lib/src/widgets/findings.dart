import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../l10n/app_localizations.dart';
import '../rust/api/dto.dart';
import '../state/analysis.dart';

/// Colour and label for a severity.
(Color, String, IconData) severityStyle(BuildContext context, String s) {
  final l = AppLocalizations.of(context);
  final dark = Theme.of(context).brightness == Brightness.dark;
  return switch (s) {
    'high' => (dark ? const Color(0xFFFF8A80) : const Color(0xFFB3261E), l.severityHigh, Icons.error_outline),
    'medium' => (dark ? const Color(0xFFFFCC80) : const Color(0xFF9A5B00), l.severityMedium, Icons.warning_amber),
    'low' => (dark ? const Color(0xFF90CAF9) : const Color(0xFF1F5FA8), l.severityLow, Icons.info_outline),
    _ => (Theme.of(context).colorScheme.outline, l.severityInfo, Icons.notes),
  };
}

/// List of findings with a summary header and the disclaimer.
class FindingsPanel extends ConsumerWidget {
  const FindingsPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final a = ref.watch(analysisProvider);
    final findings = a.findings;
    final failed = [for (final m in a.report?.modules ?? const <ModuleRunDto>[]) if (m.status == 'failed') m];
    int count(String s) => findings.where((f) => f.severity == s).length;
    final theme = Theme.of(context);
    return ListView(
      padding: const EdgeInsets.all(12),
      children: [
        Card(
          color: theme.colorScheme.secondaryContainer,
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Row(
              children: [
                Icon(Icons.balance, color: theme.colorScheme.onSecondaryContainer),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(l.disclaimer, style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSecondaryContainer)),
                ),
              ],
            ),
          ),
        ),
        if (a.report != null)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 8, horizontal: 4),
            child: Text(
              l.summaryCounts(count('high'), count('medium'), count('low'), count('info')),
              style: theme.textTheme.titleSmall,
            ),
          ),
        for (final m in failed)
          Card(
            color: theme.colorScheme.errorContainer,
            child: ListTile(
              leading: const Icon(Icons.report_problem_outlined),
              title: Text(l.moduleFailed(m.name, m.error ?? '?')),
            ),
          ),
        if (a.report != null && findings.isEmpty) Padding(padding: const EdgeInsets.all(16), child: Text(l.noFindings)),
        for (final f in findings) FindingCard(finding: f),
      ],
    );
  }
}

/// One expandable finding.
class FindingCard extends StatelessWidget {
  const FindingCard({super.key, required this.finding});
  final FindingDto finding;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final (color, label, icon) = severityStyle(context, finding.severity);
    final theme = Theme.of(context);
    final pct = (finding.confidence * 100).round();
    return Card(
      clipBehavior: Clip.antiAlias,
      child: Semantics(
        label: '$label. ${finding.title}. ${l.confidence(pct)}',
        child: ExpansionTile(
          leading: Icon(icon, color: color),
          title: Text(finding.title),
          subtitle: Row(
            children: [
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
                decoration: BoxDecoration(border: Border.all(color: color), borderRadius: BorderRadius.circular(4)),
                child: Text(label, style: theme.textTheme.labelSmall?.copyWith(color: color)),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Tooltip(
                  message: l.confidence(pct),
                  child: LinearProgressIndicator(value: finding.confidence, color: color, minHeight: 4),
                ),
              ),
              const SizedBox(width: 8),
              Text('$pct%', style: theme.textTheme.labelSmall),
            ],
          ),
          childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
          expandedCrossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (finding.explanation.isNotEmpty) ...[
              Text(l.whatItMeans, style: theme.textTheme.labelLarge),
              const SizedBox(height: 4),
              SelectableText(finding.explanation),
              const SizedBox(height: 8),
            ],
            if (finding.limitations.isNotEmpty) ...[
              Text(l.limitations, style: theme.textTheme.labelLarge),
              const SizedBox(height: 4),
              SelectableText(finding.limitations, style: theme.textTheme.bodySmall),
              const SizedBox(height: 8),
            ],
            if (finding.evidence.isNotEmpty) ...[
              Text(l.evidence, style: theme.textTheme.labelLarge),
              const SizedBox(height: 4),
              for (final e in finding.evidence)
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 2),
                  child: SelectableText.rich(
                    TextSpan(children: [
                      TextSpan(text: '${e.key}: ', style: const TextStyle(fontWeight: FontWeight.w600)),
                      TextSpan(text: e.value),
                    ]),
                    style: theme.textTheme.bodySmall,
                  ),
                ),
            ],
            Align(
              alignment: Alignment.centerRight,
              child: Text('${finding.moduleId} · ${finding.code}', style: theme.textTheme.labelSmall?.copyWith(color: theme.colorScheme.outline)),
            ),
          ],
        ),
      ),
    );
  }
}
