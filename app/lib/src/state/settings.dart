import 'dart:convert';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:path_provider/path_provider.dart';

/// User settings, persisted as JSON in the app-private support directory.
@immutable
class Settings {
  const Settings({
    this.themeMode = ThemeMode.system,
    this.locale,
    required this.maxMegapixels,
    this.elaQuality = 90,
    this.disabledModules = const {},
  });

  final ThemeMode themeMode;
  final Locale? locale;

  /// 0 = full resolution.
  final int maxMegapixels;
  final int elaQuality;
  final Set<String> disabledModules;

  static int get defaultMegapixels => Platform.isAndroid ? 12 : 0;

  Settings copyWith({
    ThemeMode? themeMode,
    Locale? Function()? locale,
    int? maxMegapixels,
    int? elaQuality,
    Set<String>? disabledModules,
  }) => Settings(
    themeMode: themeMode ?? this.themeMode,
    locale: locale != null ? locale() : this.locale,
    maxMegapixels: maxMegapixels ?? this.maxMegapixels,
    elaQuality: elaQuality ?? this.elaQuality,
    disabledModules: disabledModules ?? this.disabledModules,
  );

  Map<String, Object?> toJson() => {
    'themeMode': themeMode.name,
    'locale': locale?.languageCode,
    'maxMegapixels': maxMegapixels,
    'elaQuality': elaQuality,
    'disabledModules': disabledModules.toList(),
  };

  static Settings fromJson(Map<String, Object?> j) => Settings(
    themeMode: ThemeMode.values.firstWhere(
      (m) => m.name == j['themeMode'],
      orElse: () => ThemeMode.system,
    ),
    locale: j['locale'] is String ? Locale(j['locale']! as String) : null,
    maxMegapixels: (j['maxMegapixels'] as num?)?.toInt() ?? defaultMegapixels,
    elaQuality: (j['elaQuality'] as num?)?.toInt() ?? 90,
    disabledModules: {
      ...((j['disabledModules'] as List?) ?? const []).whereType<String>(),
    },
  );
}

/// Settings state.
class SettingsNotifier extends Notifier<Settings> {
  @override
  Settings build() {
    _load();
    return Settings(maxMegapixels: Settings.defaultMegapixels);
  }

  Future<File> _file() async {
    final dir = await getApplicationSupportDirectory();
    return File('${dir.path}/settings.json');
  }

  Future<void> _load() async {
    try {
      final f = await _file();
      if (await f.exists()) {
        state = Settings.fromJson(
          jsonDecode(await f.readAsString()) as Map<String, Object?>,
        );
      }
    } catch (_) {
      // Corrupt or unreadable settings: keep defaults.
    }
  }

  Future<void> update(Settings Function(Settings) change) async {
    state = change(state);
    try {
      final f = await _file();
      await f.parent.create(recursive: true);
      await f.writeAsString(jsonEncode(state.toJson()));
    } catch (_) {
      // Non-fatal.
    }
  }
}

final settingsProvider = NotifierProvider<SettingsNotifier, Settings>(
  SettingsNotifier.new,
);
