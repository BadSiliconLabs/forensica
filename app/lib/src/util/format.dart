/// Formatting helpers.
String formatBytes(int bytes) {
  const units = ['B', 'KB', 'MB', 'GB'];
  var v = bytes.toDouble();
  var i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return i == 0 ? '$bytes B' : '${v.toStringAsFixed(1)} ${units[i]}';
}

/// Short SHA for headers.
String shortSha(String sha) => sha.length > 16
    ? '${sha.substring(0, 8)}…${sha.substring(sha.length - 8)}'
    : sha;

/// Severity ordering (higher = more severe).
int severityRank(String s) => switch (s) {
  'high' => 3,
  'medium' => 2,
  'low' => 1,
  _ => 0,
};
