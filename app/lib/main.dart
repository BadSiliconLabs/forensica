import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import 'src/app.dart';
import 'src/platform/intake.dart';
import 'src/rust/frb_generated.dart';

Future<void> main(List<String> args) async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  await Intake.cleanTemporaryFiles();
  // Linux: `forensica <file>` (desktop "Open with").
  runApp(ProviderScope(child: ForensicaApp(initialFile: args.isNotEmpty ? args.first : null)));
}
