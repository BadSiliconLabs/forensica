import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_it.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('it'),
  ];

  /// No description provided for @appTitle.
  ///
  /// In en, this message translates to:
  /// **'Forensica'**
  String get appTitle;

  /// No description provided for @tagline.
  ///
  /// In en, this message translates to:
  /// **'Offline photo forensics. Indicators, not verdicts.'**
  String get tagline;

  /// No description provided for @openImage.
  ///
  /// In en, this message translates to:
  /// **'Open image'**
  String get openImage;

  /// No description provided for @dropHere.
  ///
  /// In en, this message translates to:
  /// **'Drop an image here'**
  String get dropHere;

  /// No description provided for @privacyNote.
  ///
  /// In en, this message translates to:
  /// **'Everything runs on this device. Nothing is uploaded. The original file is never modified.'**
  String get privacyNote;

  /// No description provided for @settings.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get settings;

  /// No description provided for @analyzing.
  ///
  /// In en, this message translates to:
  /// **'Analyzing… {completed}/{total}'**
  String analyzing(int completed, int total);

  /// No description provided for @cancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get cancel;

  /// No description provided for @rerun.
  ///
  /// In en, this message translates to:
  /// **'Run again'**
  String get rerun;

  /// No description provided for @exportPdf.
  ///
  /// In en, this message translates to:
  /// **'Export PDF report'**
  String get exportPdf;

  /// No description provided for @exportJson.
  ///
  /// In en, this message translates to:
  /// **'Export JSON report'**
  String get exportJson;

  /// No description provided for @closeFile.
  ///
  /// In en, this message translates to:
  /// **'Close file'**
  String get closeFile;

  /// No description provided for @findings.
  ///
  /// In en, this message translates to:
  /// **'Findings'**
  String get findings;

  /// No description provided for @metadata.
  ///
  /// In en, this message translates to:
  /// **'Metadata'**
  String get metadata;

  /// No description provided for @tools.
  ///
  /// In en, this message translates to:
  /// **'Tools'**
  String get tools;

  /// No description provided for @image.
  ///
  /// In en, this message translates to:
  /// **'Image'**
  String get image;

  /// No description provided for @noFindings.
  ///
  /// In en, this message translates to:
  /// **'No indicators were reported by the enabled modules. This does not prove the image is unaltered.'**
  String get noFindings;

  /// No description provided for @disclaimer.
  ///
  /// In en, this message translates to:
  /// **'These are indicators, not verdicts. Benign processing can trigger them and skilled manipulation can evade them. Always read the limitations.'**
  String get disclaimer;

  /// No description provided for @severityHigh.
  ///
  /// In en, this message translates to:
  /// **'High'**
  String get severityHigh;

  /// No description provided for @severityMedium.
  ///
  /// In en, this message translates to:
  /// **'Medium'**
  String get severityMedium;

  /// No description provided for @severityLow.
  ///
  /// In en, this message translates to:
  /// **'Low'**
  String get severityLow;

  /// No description provided for @severityInfo.
  ///
  /// In en, this message translates to:
  /// **'Info'**
  String get severityInfo;

  /// No description provided for @confidence.
  ///
  /// In en, this message translates to:
  /// **'Confidence {value}%'**
  String confidence(int value);

  /// No description provided for @whatItMeans.
  ///
  /// In en, this message translates to:
  /// **'What it means'**
  String get whatItMeans;

  /// No description provided for @limitations.
  ///
  /// In en, this message translates to:
  /// **'Limitations'**
  String get limitations;

  /// No description provided for @evidence.
  ///
  /// In en, this message translates to:
  /// **'Evidence'**
  String get evidence;

  /// No description provided for @moduleFailed.
  ///
  /// In en, this message translates to:
  /// **'{module} failed: {error}'**
  String moduleFailed(String module, String error);

  /// No description provided for @searchMetadata.
  ///
  /// In en, this message translates to:
  /// **'Search metadata'**
  String get searchMetadata;

  /// No description provided for @noMetadata.
  ///
  /// In en, this message translates to:
  /// **'No metadata found.'**
  String get noMetadata;

  /// No description provided for @overlayOpacity.
  ///
  /// In en, this message translates to:
  /// **'Overlay opacity'**
  String get overlayOpacity;

  /// No description provided for @modeOverlay.
  ///
  /// In en, this message translates to:
  /// **'Overlay'**
  String get modeOverlay;

  /// No description provided for @modeSideBySide.
  ///
  /// In en, this message translates to:
  /// **'Side by side'**
  String get modeSideBySide;

  /// No description provided for @modeSwipe.
  ///
  /// In en, this message translates to:
  /// **'Swipe'**
  String get modeSwipe;

  /// No description provided for @pixelInspector.
  ///
  /// In en, this message translates to:
  /// **'Pixel inspector'**
  String get pixelInspector;

  /// No description provided for @tapToInspect.
  ///
  /// In en, this message translates to:
  /// **'Tap the image to inspect a pixel.'**
  String get tapToInspect;

  /// No description provided for @layers.
  ///
  /// In en, this message translates to:
  /// **'Layers'**
  String get layers;

  /// No description provided for @originalImage.
  ///
  /// In en, this message translates to:
  /// **'Original'**
  String get originalImage;

  /// No description provided for @sha256.
  ///
  /// In en, this message translates to:
  /// **'SHA-256'**
  String get sha256;

  /// No description provided for @copied.
  ///
  /// In en, this message translates to:
  /// **'Copied to clipboard'**
  String get copied;

  /// No description provided for @saved.
  ///
  /// In en, this message translates to:
  /// **'Saved: {path}'**
  String saved(String path);

  /// No description provided for @exportFailed.
  ///
  /// In en, this message translates to:
  /// **'Export failed: {error}'**
  String exportFailed(String error);

  /// No description provided for @openFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not open the file: {error}'**
  String openFailed(String error);

  /// No description provided for @theme.
  ///
  /// In en, this message translates to:
  /// **'Theme'**
  String get theme;

  /// No description provided for @themeSystem.
  ///
  /// In en, this message translates to:
  /// **'System'**
  String get themeSystem;

  /// No description provided for @themeLight.
  ///
  /// In en, this message translates to:
  /// **'Light'**
  String get themeLight;

  /// No description provided for @themeDark.
  ///
  /// In en, this message translates to:
  /// **'Dark'**
  String get themeDark;

  /// No description provided for @language.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get language;

  /// No description provided for @languageSystem.
  ///
  /// In en, this message translates to:
  /// **'System default'**
  String get languageSystem;

  /// No description provided for @performance.
  ///
  /// In en, this message translates to:
  /// **'Performance'**
  String get performance;

  /// No description provided for @maxResolution.
  ///
  /// In en, this message translates to:
  /// **'Maximum analysis resolution'**
  String get maxResolution;

  /// No description provided for @fullResolution.
  ///
  /// In en, this message translates to:
  /// **'Full resolution'**
  String get fullResolution;

  /// No description provided for @megapixels.
  ///
  /// In en, this message translates to:
  /// **'{mp} MP'**
  String megapixels(int mp);

  /// No description provided for @degradedModeHint.
  ///
  /// In en, this message translates to:
  /// **'Lower values are faster and use less memory, but small traces may be missed. Metadata is always analysed in full.'**
  String get degradedModeHint;

  /// No description provided for @modules.
  ///
  /// In en, this message translates to:
  /// **'Analysis modules'**
  String get modules;

  /// No description provided for @about.
  ///
  /// In en, this message translates to:
  /// **'About'**
  String get about;

  /// No description provided for @aboutText.
  ///
  /// In en, this message translates to:
  /// **'Forensica {version}. Free software under AGPL-3.0-or-later. No network access, no telemetry.'**
  String aboutText(String version);

  /// No description provided for @elaQuality.
  ///
  /// In en, this message translates to:
  /// **'ELA quality: {q}'**
  String elaQuality(int q);

  /// No description provided for @render.
  ///
  /// In en, this message translates to:
  /// **'Render'**
  String get render;

  /// No description provided for @channel.
  ///
  /// In en, this message translates to:
  /// **'Channel'**
  String get channel;

  /// No description provided for @bitPlane.
  ///
  /// In en, this message translates to:
  /// **'Bit plane: {bit}'**
  String bitPlane(int bit);

  /// No description provided for @summaryCounts.
  ///
  /// In en, this message translates to:
  /// **'{high} high · {medium} medium · {low} low · {info} info'**
  String summaryCounts(int high, int medium, int low, int info);

  /// No description provided for @otherViews.
  ///
  /// In en, this message translates to:
  /// **'Other views'**
  String get otherViews;

  /// No description provided for @batch.
  ///
  /// In en, this message translates to:
  /// **'Batch analysis'**
  String get batch;

  /// No description provided for @batchHint.
  ///
  /// In en, this message translates to:
  /// **'Analyse every image in a folder and write a JSON and PDF report for each.'**
  String get batchHint;

  /// No description provided for @chooseInputFolder.
  ///
  /// In en, this message translates to:
  /// **'Choose input folder'**
  String get chooseInputFolder;

  /// No description provided for @chooseOutputFolder.
  ///
  /// In en, this message translates to:
  /// **'Choose output folder'**
  String get chooseOutputFolder;

  /// No description provided for @start.
  ///
  /// In en, this message translates to:
  /// **'Start'**
  String get start;

  /// No description provided for @batchDone.
  ///
  /// In en, this message translates to:
  /// **'Done: {ok} analysed, {failed} failed.'**
  String batchDone(int ok, int failed);

  /// No description provided for @fileDetails.
  ///
  /// In en, this message translates to:
  /// **'File details'**
  String get fileDetails;

  /// No description provided for @format.
  ///
  /// In en, this message translates to:
  /// **'Format'**
  String get format;

  /// No description provided for @size.
  ///
  /// In en, this message translates to:
  /// **'Size'**
  String get size;

  /// No description provided for @dimensions.
  ///
  /// In en, this message translates to:
  /// **'Dimensions'**
  String get dimensions;

  /// No description provided for @extensionMismatch.
  ///
  /// In en, this message translates to:
  /// **'Extension does not match content'**
  String get extensionMismatch;

  /// No description provided for @notDecodable.
  ///
  /// In en, this message translates to:
  /// **'Pixel analysis is not available for this format; only metadata is analysed.'**
  String get notDecodable;

  /// No description provided for @visualizations.
  ///
  /// In en, this message translates to:
  /// **'Visualizations'**
  String get visualizations;

  /// No description provided for @selectLayerHint.
  ///
  /// In en, this message translates to:
  /// **'Select a layer to overlay it on the image.'**
  String get selectLayerHint;

  /// No description provided for @duplicates.
  ///
  /// In en, this message translates to:
  /// **'Near-duplicates'**
  String get duplicates;

  /// No description provided for @duplicatesHint.
  ///
  /// In en, this message translates to:
  /// **'Search a folder on this device for visually similar images (perceptual hash).'**
  String get duplicatesHint;

  /// No description provided for @chooseFolder.
  ///
  /// In en, this message translates to:
  /// **'Choose folder'**
  String get chooseFolder;

  /// No description provided for @noDuplicates.
  ///
  /// In en, this message translates to:
  /// **'No similar images found.'**
  String get noDuplicates;

  /// No description provided for @prnu.
  ///
  /// In en, this message translates to:
  /// **'Camera fingerprint (PRNU)'**
  String get prnu;

  /// No description provided for @prnuHint.
  ///
  /// In en, this message translates to:
  /// **'Compare this image\'s sensor noise with reference photos known to come from one camera (at least 5, same resolution).'**
  String get prnuHint;

  /// No description provided for @chooseReferences.
  ///
  /// In en, this message translates to:
  /// **'Choose reference photos'**
  String get chooseReferences;

  /// No description provided for @legend.
  ///
  /// In en, this message translates to:
  /// **'Legend'**
  String get legend;

  /// No description provided for @experimental.
  ///
  /// In en, this message translates to:
  /// **'Experimental'**
  String get experimental;

  /// No description provided for @copySha.
  ///
  /// In en, this message translates to:
  /// **'Copy SHA-256'**
  String get copySha;

  /// No description provided for @moduleTimes.
  ///
  /// In en, this message translates to:
  /// **'Module timings'**
  String get moduleTimes;

  /// No description provided for @cached.
  ///
  /// In en, this message translates to:
  /// **'cached'**
  String get cached;

  /// No description provided for @prnuMatch.
  ///
  /// In en, this message translates to:
  /// **'PCE {pce} ≥ {threshold}: the sensor noise matches the reference camera.'**
  String prnuMatch(String pce, String threshold);

  /// No description provided for @prnuNoMatch.
  ///
  /// In en, this message translates to:
  /// **'PCE {pce} < {threshold}: no match. Inconclusive if the image was resized, cropped or heavily compressed.'**
  String prnuNoMatch(String pce, String threshold);

  /// No description provided for @prnuDetails.
  ///
  /// In en, this message translates to:
  /// **'{refs} reference(s) used, {skipped} skipped (different resolution or unreadable), {crop}×{crop} crop.'**
  String prnuDetails(int refs, int skipped, int crop);

  /// No description provided for @chooseImages.
  ///
  /// In en, this message translates to:
  /// **'Choose images'**
  String get chooseImages;

  /// No description provided for @similarItem.
  ///
  /// In en, this message translates to:
  /// **'distance {d} · {w}×{h}'**
  String similarItem(int d, int w, int h);

  /// No description provided for @searching.
  ///
  /// In en, this message translates to:
  /// **'Searching…'**
  String get searching;

  /// No description provided for @openThis.
  ///
  /// In en, this message translates to:
  /// **'Analyse this file'**
  String get openThis;

  /// No description provided for @exportReport.
  ///
  /// In en, this message translates to:
  /// **'Export report'**
  String get exportReport;

  /// No description provided for @viewerLabel.
  ///
  /// In en, this message translates to:
  /// **'Image {name}, showing {layer}'**
  String viewerLabel(String name, String layer);
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'it'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'it':
      return AppLocalizationsIt();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
