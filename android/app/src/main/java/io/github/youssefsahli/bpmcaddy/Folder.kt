package io.github.youssefsahli.bpmcaddy

import android.content.Context
import android.net.Uri
import androidx.documentfile.provider.DocumentFile
import java.io.File
import uniffi.bpm_caddy_mobile.Caddy

/**
 * Le dossier d'échange vu du téléphone. Android ne donne pas de chemin,
 * seulement des documents : les fichiers `.bpmposte` des postes sont
 * recopiés dans le cache privé, la bibliothèque y fait ce qu'elle fait
 * sur le bureau (déposer le sien, lire les autres), et le fichier de ce
 * téléphone est recopié dans le dossier choisi. Rien n'y est lisible : ce
 * sont des enregistrements scellés.
 */
object Folder {
    fun exchange(context: Context, caddy: Caddy, tree: String, today: String): String {
        val root = DocumentFile.fromTreeUri(context, Uri.parse(tree))
            ?: return T("mobile_folder_none")
        val local = File(context.cacheDir, "echange").apply {
            deleteRecursively()
            mkdirs()
        }
        root.listFiles()
            .filter { it.isFile && (it.name ?: "").endsWith(".bpmposte") }
            .forEach { doc ->
                context.contentResolver.openInputStream(doc.uri)?.use { input ->
                    File(local, doc.name!!).outputStream().use { input.copyTo(it) }
                }
            }
        val done = caddy.exchangeFolder(local.absolutePath, today)
        val mine = File(local, done.mine)
        if (mine.exists()) {
            val target = root.findFile(done.mine)
                ?: root.createFile("application/octet-stream", done.mine)
            target?.let { doc ->
                // « wt » : réécrit en entier, jamais ajouté à la suite.
                context.contentResolver.openOutputStream(doc.uri, "wt")?.use { out ->
                    mine.inputStream().use { it.copyTo(out) }
                }
            }
        }
        local.deleteRecursively()
        return done.said
    }
}
