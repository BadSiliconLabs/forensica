import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';

/// File intake from the platform: Android "Open with"/"Share" intents and temp-file hygiene.
///
/// On Android the native side copies the shared content URI into
/// `<cache>/intake/` (app-private) and hands Dart a path. Those copies are deleted
/// when the file is closed and on every start.
class Intake {
  static const _channel = MethodChannel('org.forensica/intake');

  /// Calls [onFile] for the launch intent and for every later share/open.
  static void listen(Future<void> Function(String path) onFile) {
    if (!Platform.isAndroid) return;
    _channel.setMethodCallHandler((call) async {
      if (call.method == 'fileReceived' && call.arguments is String) {
        await onFile(call.arguments as String);
      }
    });
    _channel.invokeMethod<String>('getInitialFile').then((p) {
      if (p != null) onFile(p);
    }).catchError((Object _) {});
  }

  static Future<Directory?> _intakeDir() async {
    if (!Platform.isAndroid) return null;
    final c = await getTemporaryDirectory();
    return Directory('${c.path}/intake');
  }

  /// Deletes leftovers from previous runs.
  static Future<void> cleanTemporaryFiles() async {
    try {
      final d = await _intakeDir();
      if (d != null && await d.exists()) await d.delete(recursive: true);
      if (Platform.isAndroid) await FilePicker.clearTemporaryFiles();
    } catch (_) {
      // Best effort.
    }
  }

  /// Deletes [path] if it is a temporary copy we own (never the user's original).
  static Future<void> deleteIfTemporary(String path) async {
    try {
      if (!Platform.isAndroid) return;
      final tmp = await getTemporaryDirectory();
      if (path.startsWith(tmp.path)) {
        final f = File(path);
        if (await f.exists()) await f.delete();
      }
    } catch (_) {
      // Best effort.
    }
  }
}
