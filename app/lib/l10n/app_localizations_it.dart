// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Italian (`it`).
class AppLocalizationsIt extends AppLocalizations {
  AppLocalizationsIt([String locale = 'it']) : super(locale);

  @override
  String get appTitle => 'Forensica';

  @override
  String get tagline =>
      'Analisi forense delle foto, offline. Indizi, non verdetti.';

  @override
  String get openImage => 'Apri immagine';

  @override
  String get dropHere => 'Trascina qui un\'immagine';

  @override
  String get privacyNote =>
      'Tutto avviene su questo dispositivo. Nulla viene caricato in rete. Il file originale non viene mai modificato.';

  @override
  String get settings => 'Impostazioni';

  @override
  String analyzing(int completed, int total) {
    return 'Analisi in corso… $completed/$total';
  }

  @override
  String get cancel => 'Annulla';

  @override
  String get rerun => 'Ripeti analisi';

  @override
  String get exportPdf => 'Esporta report PDF';

  @override
  String get exportJson => 'Esporta report JSON';

  @override
  String get closeFile => 'Chiudi file';

  @override
  String get findings => 'Risultati';

  @override
  String get metadata => 'Metadati';

  @override
  String get tools => 'Strumenti';

  @override
  String get image => 'Immagine';

  @override
  String get noFindings =>
      'I moduli attivi non hanno segnalato indizi. Questo non dimostra che l\'immagine sia inalterata.';

  @override
  String get disclaimer =>
      'Questi sono indizi, non verdetti. Elaborazioni innocue possono attivarli e manipolazioni abili possono eluderli. Leggi sempre i limiti.';

  @override
  String get severityHigh => 'Alta';

  @override
  String get severityMedium => 'Media';

  @override
  String get severityLow => 'Bassa';

  @override
  String get severityInfo => 'Info';

  @override
  String confidence(int value) {
    return 'Confidenza $value%';
  }

  @override
  String get whatItMeans => 'Cosa significa';

  @override
  String get limitations => 'Limiti';

  @override
  String get evidence => 'Evidenze';

  @override
  String moduleFailed(String module, String error) {
    return '$module non riuscito: $error';
  }

  @override
  String get searchMetadata => 'Cerca nei metadati';

  @override
  String get noMetadata => 'Nessun metadato trovato.';

  @override
  String get overlayOpacity => 'Opacità sovrapposizione';

  @override
  String get modeOverlay => 'Sovrapponi';

  @override
  String get modeSideBySide => 'Affiancate';

  @override
  String get modeSwipe => 'Scorrimento';

  @override
  String get pixelInspector => 'Ispettore pixel';

  @override
  String get tapToInspect => 'Tocca l\'immagine per ispezionare un pixel.';

  @override
  String get layers => 'Livelli';

  @override
  String get originalImage => 'Originale';

  @override
  String get sha256 => 'SHA-256';

  @override
  String get copied => 'Copiato negli appunti';

  @override
  String saved(String path) {
    return 'Salvato: $path';
  }

  @override
  String exportFailed(String error) {
    return 'Esportazione non riuscita: $error';
  }

  @override
  String openFailed(String error) {
    return 'Impossibile aprire il file: $error';
  }

  @override
  String get theme => 'Tema';

  @override
  String get themeSystem => 'Sistema';

  @override
  String get themeLight => 'Chiaro';

  @override
  String get themeDark => 'Scuro';

  @override
  String get language => 'Lingua';

  @override
  String get languageSystem => 'Predefinita di sistema';

  @override
  String get performance => 'Prestazioni';

  @override
  String get maxResolution => 'Risoluzione massima di analisi';

  @override
  String get fullResolution => 'Risoluzione piena';

  @override
  String megapixels(int mp) {
    return '$mp MP';
  }

  @override
  String get degradedModeHint =>
      'Valori più bassi sono più veloci e usano meno memoria, ma tracce piccole possono sfuggire. I metadati sono sempre analizzati per intero.';

  @override
  String get modules => 'Moduli di analisi';

  @override
  String get about => 'Informazioni';

  @override
  String aboutText(String version) {
    return 'Forensica $version. Software libero con licenza AGPL-3.0-or-later. Nessun accesso alla rete, nessuna telemetria.';
  }

  @override
  String elaQuality(int q) {
    return 'Qualità ELA: $q';
  }

  @override
  String get render => 'Genera';

  @override
  String get channel => 'Canale';

  @override
  String bitPlane(int bit) {
    return 'Piano di bit: $bit';
  }

  @override
  String summaryCounts(int high, int medium, int low, int info) {
    return '$high alte · $medium medie · $low basse · $info info';
  }

  @override
  String get otherViews => 'Altre viste';

  @override
  String get batch => 'Analisi in blocco';

  @override
  String get batchHint =>
      'Analizza tutte le immagini di una cartella e scrive un report JSON e PDF per ciascuna.';

  @override
  String get chooseInputFolder => 'Scegli cartella di input';

  @override
  String get chooseOutputFolder => 'Scegli cartella di output';

  @override
  String get start => 'Avvia';

  @override
  String batchDone(int ok, int failed) {
    return 'Completato: $ok analizzate, $failed non riuscite.';
  }

  @override
  String get fileDetails => 'Dettagli file';

  @override
  String get format => 'Formato';

  @override
  String get size => 'Dimensione';

  @override
  String get dimensions => 'Risoluzione';

  @override
  String get extensionMismatch => 'L\'estensione non corrisponde al contenuto';

  @override
  String get notDecodable =>
      'L\'analisi dei pixel non è disponibile per questo formato; vengono analizzati solo i metadati.';

  @override
  String get visualizations => 'Visualizzazioni';

  @override
  String get selectLayerHint =>
      'Seleziona un livello da sovrapporre all\'immagine.';

  @override
  String get duplicates => 'Quasi-duplicati';

  @override
  String get duplicatesHint =>
      'Cerca in una cartella di questo dispositivo immagini visivamente simili (hash percettivo).';

  @override
  String get chooseFolder => 'Scegli cartella';

  @override
  String get noDuplicates => 'Nessuna immagine simile trovata.';

  @override
  String get prnu => 'Impronta del sensore (PRNU)';

  @override
  String get prnuHint =>
      'Confronta il rumore del sensore di questa immagine con foto di riferimento scattate dalla stessa fotocamera (almeno 5, stessa risoluzione).';

  @override
  String get chooseReferences => 'Scegli foto di riferimento';

  @override
  String get legend => 'Legenda';

  @override
  String get experimental => 'Sperimentale';

  @override
  String get copySha => 'Copia SHA-256';

  @override
  String get moduleTimes => 'Tempi dei moduli';

  @override
  String get cached => 'in cache';
}
