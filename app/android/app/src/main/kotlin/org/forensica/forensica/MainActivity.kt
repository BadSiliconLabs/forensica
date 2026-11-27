package org.forensica.forensica

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.provider.OpenableColumns
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import java.io.File
import java.util.concurrent.Executors

/**
 * Receives "Open with" (ACTION_VIEW) and "Share" (ACTION_SEND) intents.
 * The shared content is copied read-only into the app-private cache
 * (`cache/intake/`), never written back, and deleted by the Dart side
 * when the file is closed or on the next start.
 */
class MainActivity : FlutterActivity() {
    private val channelName = "org.forensica/intake"
    private var channel: MethodChannel? = null
    private var pendingIntent: Intent? = null
    private val io = Executors.newSingleThreadExecutor()
    private val main = Handler(Looper.getMainLooper())

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        pendingIntent = intent
        channel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, channelName).also { ch ->
            ch.setMethodCallHandler { call, result ->
                when (call.method) {
                    "getInitialFile" -> {
                        val i = pendingIntent
                        pendingIntent = null
                        val uri = i?.let { sharedUri(it) }
                        if (uri == null) result.success(null) else copyAsync(uri) { result.success(it) }
                    }
                    else -> result.notImplemented()
                }
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        val uri = sharedUri(intent) ?: return
        copyAsync(uri) { path -> if (path != null) channel?.invokeMethod("fileReceived", path) }
    }

    @Suppress("DEPRECATION")
    private fun sharedUri(i: Intent): Uri? = when (i.action) {
        Intent.ACTION_VIEW -> i.data
        Intent.ACTION_SEND ->
            if (Build.VERSION.SDK_INT >= 33) i.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java)
            else i.getParcelableExtra(Intent.EXTRA_STREAM) as? Uri
        else -> null
    }

    private fun displayName(uri: Uri): String {
        var name: String? = null
        try {
            contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
                if (c.moveToFirst()) name = c.getString(0)
            }
        } catch (_: Exception) {
        }
        val raw = name ?: uri.lastPathSegment ?: "shared-image"
        // Keep only a safe file name component.
        return raw.substringAfterLast('/').replace(Regex("[^A-Za-z0-9._ -]"), "_").take(120).ifEmpty { "shared-image" }
    }

    private fun copyAsync(uri: Uri, done: (String?) -> Unit) {
        io.execute {
            val path = try {
                val dir = File(cacheDir, "intake").apply { mkdirs() }
                val out = File(dir, "${System.currentTimeMillis()}-${displayName(uri)}")
                contentResolver.openInputStream(uri)?.use { input ->
                    out.outputStream().use { input.copyTo(it) }
                }
                if (out.exists() && out.length() > 0) out.absolutePath else null
            } catch (_: Exception) {
                null
            }
            main.post { done(path) }
        }
    }
}
