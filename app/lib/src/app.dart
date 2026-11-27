import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

import '../l10n/app_localizations.dart';
import 'platform/intake.dart';
import 'screens/analysis_screen.dart';
import 'screens/batch_screen.dart';
import 'screens/home_screen.dart';
import 'screens/settings_screen.dart';
import 'state/analysis.dart';
import 'state/settings.dart';

final _router = GoRouter(
  routes: [
    GoRoute(path: '/', builder: (context, state) => const HomeScreen()),
    GoRoute(
      path: '/analysis',
      builder: (context, state) => const AnalysisScreen(),
    ),
    GoRoute(
      path: '/settings',
      builder: (context, state) => const SettingsScreen(),
    ),
    GoRoute(path: '/batch', builder: (context, state) => const BatchScreen()),
  ],
);

/// Root widget.
class ForensicaApp extends ConsumerStatefulWidget {
  const ForensicaApp({super.key, this.initialFile});

  /// File passed on the command line (Linux).
  final String? initialFile;

  @override
  ConsumerState<ForensicaApp> createState() => _ForensicaAppState();
}

class _ForensicaAppState extends ConsumerState<ForensicaApp> {
  @override
  void initState() {
    super.initState();
    final f = widget.initialFile;
    if (f != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        await ref.read(analysisProvider.notifier).open(f);
        _router.go('/analysis');
      });
    }
    // Android "Open with" / "Share" intents.
    Intake.listen((path) async {
      await ref.read(analysisProvider.notifier).open(path);
      _router.go('/analysis');
    });
  }

  @override
  Widget build(BuildContext context) {
    final settings = ref.watch(settingsProvider);
    const seed = Color(0xFF3F6E8C);
    return MaterialApp.router(
      onGenerateTitle: (c) => AppLocalizations.of(c).appTitle,
      debugShowCheckedModeBanner: false,
      theme: ThemeData(colorSchemeSeed: seed, brightness: Brightness.light),
      darkTheme: ThemeData(colorSchemeSeed: seed, brightness: Brightness.dark),
      themeMode: settings.themeMode,
      locale: settings.locale,
      supportedLocales: AppLocalizations.supportedLocales,
      localizationsDelegates: const [
        AppLocalizations.delegate,
        ...GlobalMaterialLocalizations.delegates,
      ],
      routerConfig: _router,
    );
  }
}
