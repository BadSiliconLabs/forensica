import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../l10n/app_localizations.dart';
import '../rust/api/engine.dart' as engine;
import '../state/settings.dart';

/// Settings: theme, language, performance, modules, about.
class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  static const _mpChoices = [0, 48, 24, 12, 8, 4];

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final s = ref.watch(settingsProvider);
    final n = ref.read(settingsProvider.notifier);
    final modules = engine.listModules();
    final theme = Theme.of(context);
    Widget header(String t) => Padding(
      padding: const EdgeInsets.fromLTRB(16, 24, 16, 8),
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
    return Scaffold(
      appBar: AppBar(title: Text(l.settings)),
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 760),
          child: ListView(
            children: [
              header(l.theme),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: SegmentedButton<ThemeMode>(
                  segments: [
                    ButtonSegment(
                      value: ThemeMode.system,
                      label: Text(l.themeSystem),
                    ),
                    ButtonSegment(
                      value: ThemeMode.light,
                      label: Text(l.themeLight),
                    ),
                    ButtonSegment(
                      value: ThemeMode.dark,
                      label: Text(l.themeDark),
                    ),
                  ],
                  selected: {s.themeMode},
                  onSelectionChanged: (v) =>
                      n.update((x) => x.copyWith(themeMode: v.first)),
                ),
              ),
              header(l.language),
              RadioGroup<String>(
                groupValue: s.locale?.languageCode ?? '',
                onChanged: (v) => n.update(
                  (x) => x.copyWith(
                    locale: () => (v == null || v.isEmpty) ? null : Locale(v),
                  ),
                ),
                child: Column(
                  children: [
                    RadioListTile<String>(
                      value: '',
                      title: Text(l.languageSystem),
                    ),
                    const RadioListTile<String>(
                      value: 'en',
                      title: Text('English'),
                    ),
                    const RadioListTile<String>(
                      value: 'it',
                      title: Text('Italiano'),
                    ),
                  ],
                ),
              ),
              header(l.performance),
              ListTile(
                title: Text(l.maxResolution),
                subtitle: Text(l.degradedModeHint),
                trailing: DropdownButton<int>(
                  value: _mpChoices.contains(s.maxMegapixels)
                      ? s.maxMegapixels
                      : 0,
                  items: [
                    for (final mp in _mpChoices)
                      DropdownMenuItem(
                        value: mp,
                        child: Text(
                          mp == 0 ? l.fullResolution : l.megapixels(mp),
                        ),
                      ),
                  ],
                  onChanged: (v) =>
                      n.update((x) => x.copyWith(maxMegapixels: v)),
                ),
              ),
              header(l.modules),
              for (final m in modules)
                SwitchListTile(
                  title: Text(m.name),
                  subtitle: Text(m.description),
                  value: !s.disabledModules.contains(m.id),
                  onChanged: (on) => n.update((x) {
                    final d = {...x.disabledModules};
                    on ? d.remove(m.id) : d.add(m.id);
                    return x.copyWith(disabledModules: d);
                  }),
                ),
              header(l.about),
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 0, 16, 32),
                child: Text(l.aboutText(engine.engineVersion())),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
