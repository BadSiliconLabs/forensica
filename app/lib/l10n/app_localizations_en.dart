// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'Forensica';

  @override
  String get tagline => 'Offline photo forensics. Indicators, not verdicts.';

  @override
  String get openImage => 'Open image';

  @override
  String get dropHere => 'Drop an image here';

  @override
  String get privacyNote =>
      'Everything runs on this device. Nothing is uploaded. The original file is never modified.';

  @override
  String get settings => 'Settings';

  @override
  String analyzing(int completed, int total) {
    return 'Analyzing… $completed/$total';
  }

  @override
  String get cancel => 'Cancel';

  @override
  String get rerun => 'Run again';

  @override
  String get exportPdf => 'Export PDF report';

  @override
  String get exportJson => 'Export JSON report';

  @override
  String get closeFile => 'Close file';

  @override
  String get findings => 'Findings';

  @override
  String get metadata => 'Metadata';

  @override
  String get tools => 'Tools';

  @override
  String get image => 'Image';

  @override
  String get noFindings =>
      'No indicators were reported by the enabled modules. This does not prove the image is unaltered.';

  @override
  String get disclaimer =>
      'These are indicators, not verdicts. Benign processing can trigger them and skilled manipulation can evade them. Always read the limitations.';

  @override
  String get severityHigh => 'High';

  @override
  String get severityMedium => 'Medium';

  @override
  String get severityLow => 'Low';

  @override
  String get severityInfo => 'Info';

  @override
  String confidence(int value) {
    return 'Confidence $value%';
  }

  @override
  String get whatItMeans => 'What it means';

  @override
  String get limitations => 'Limitations';

  @override
  String get evidence => 'Evidence';

  @override
  String moduleFailed(String module, String error) {
    return '$module failed: $error';
  }

  @override
  String get searchMetadata => 'Search metadata';

  @override
  String get noMetadata => 'No metadata found.';

  @override
  String get overlayOpacity => 'Overlay opacity';

  @override
  String get modeOverlay => 'Overlay';

  @override
  String get modeSideBySide => 'Side by side';

  @override
  String get modeSwipe => 'Swipe';

  @override
  String get pixelInspector => 'Pixel inspector';

  @override
  String get tapToInspect => 'Tap the image to inspect a pixel.';

  @override
  String get layers => 'Layers';

  @override
  String get originalImage => 'Original';

  @override
  String get sha256 => 'SHA-256';

  @override
  String get copied => 'Copied to clipboard';

  @override
  String saved(String path) {
    return 'Saved: $path';
  }

  @override
  String exportFailed(String error) {
    return 'Export failed: $error';
  }

  @override
  String openFailed(String error) {
    return 'Could not open the file: $error';
  }

  @override
  String get theme => 'Theme';

  @override
  String get themeSystem => 'System';

  @override
  String get themeLight => 'Light';

  @override
  String get themeDark => 'Dark';

  @override
  String get language => 'Language';

  @override
  String get languageSystem => 'System default';

  @override
  String get performance => 'Performance';

  @override
  String get maxResolution => 'Maximum analysis resolution';

  @override
  String get fullResolution => 'Full resolution';

  @override
  String megapixels(int mp) {
    return '$mp MP';
  }

  @override
  String get degradedModeHint =>
      'Lower values are faster and use less memory, but small traces may be missed. Metadata is always analysed in full.';

  @override
  String get modules => 'Analysis modules';

  @override
  String get about => 'About';

  @override
  String aboutText(String version) {
    return 'Forensica $version. Free software under AGPL-3.0-or-later. No network access, no telemetry.';
  }

  @override
  String elaQuality(int q) {
    return 'ELA quality: $q';
  }

  @override
  String get render => 'Render';

  @override
  String get channel => 'Channel';

  @override
  String bitPlane(int bit) {
    return 'Bit plane: $bit';
  }

  @override
  String summaryCounts(int high, int medium, int low, int info) {
    return '$high high · $medium medium · $low low · $info info';
  }

  @override
  String get otherViews => 'Other views';

  @override
  String get batch => 'Batch analysis';

  @override
  String get batchHint =>
      'Analyse every image in a folder and write a JSON and PDF report for each.';

  @override
  String get chooseInputFolder => 'Choose input folder';

  @override
  String get chooseOutputFolder => 'Choose output folder';

  @override
  String get start => 'Start';

  @override
  String batchDone(int ok, int failed) {
    return 'Done: $ok analysed, $failed failed.';
  }

  @override
  String get fileDetails => 'File details';

  @override
  String get format => 'Format';

  @override
  String get size => 'Size';

  @override
  String get dimensions => 'Dimensions';

  @override
  String get extensionMismatch => 'Extension does not match content';

  @override
  String get notDecodable =>
      'Pixel analysis is not available for this format; only metadata is analysed.';

  @override
  String get visualizations => 'Visualizations';

  @override
  String get selectLayerHint => 'Select a layer to overlay it on the image.';

  @override
  String get duplicates => 'Near-duplicates';

  @override
  String get duplicatesHint =>
      'Search a folder on this device for visually similar images (perceptual hash).';

  @override
  String get chooseFolder => 'Choose folder';

  @override
  String get noDuplicates => 'No similar images found.';

  @override
  String get prnu => 'Camera fingerprint (PRNU)';

  @override
  String get prnuHint =>
      'Compare this image\'s sensor noise with reference photos known to come from one camera (at least 5, same resolution).';

  @override
  String get chooseReferences => 'Choose reference photos';

  @override
  String get legend => 'Legend';

  @override
  String get experimental => 'Experimental';

  @override
  String get copySha => 'Copy SHA-256';

  @override
  String get moduleTimes => 'Module timings';

  @override
  String get cached => 'cached';

  @override
  String prnuMatch(String pce, String threshold) {
    return 'PCE $pce ≥ $threshold: the sensor noise matches the reference camera.';
  }

  @override
  String prnuNoMatch(String pce, String threshold) {
    return 'PCE $pce < $threshold: no match. Inconclusive if the image was resized, cropped or heavily compressed.';
  }

  @override
  String prnuDetails(int refs, int skipped, int crop) {
    return '$refs reference(s) used, $skipped skipped (different resolution or unreadable), $crop×$crop crop.';
  }

  @override
  String get chooseImages => 'Choose images';

  @override
  String similarItem(int d, int w, int h) {
    return 'distance $d · $w×$h';
  }

  @override
  String get searching => 'Searching…';

  @override
  String get openThis => 'Analyse this file';

  @override
  String get exportReport => 'Export report';

  @override
  String viewerLabel(String name, String layer) {
    return 'Image $name, showing $layer';
  }
}
