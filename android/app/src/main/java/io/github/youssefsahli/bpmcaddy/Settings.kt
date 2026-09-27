package io.github.youssefsahli.bpmcaddy

import android.content.Context

/**
 * Ce que ce téléphone règle pour lui-même : qui le tient (ses initiales
 * signent ses messages), le port des postes, des adresses écrites. Rien
 * de secret : la base, elle, est dans le coffre ([Vault]).
 */
class Settings(context: Context) {
    private val prefs = context.getSharedPreferences("reglages", Context.MODE_PRIVATE)

    var initials: String
        get() = prefs.getString("initiales", "") ?: ""
        set(v) = prefs.edit().putString("initiales", v.trim().uppercase()).apply()

    var port: Int
        get() = prefs.getInt("port", DEFAULT_PORT)
        set(v) = prefs.edit().putInt("port", v).apply()

    /** Adresses `hôte:port` composées en plus des postes entendus. */
    var addresses: List<String>
        get() = (prefs.getString("adresses", "") ?: "")
            .split('\n').map { it.trim() }.filter { it.isNotEmpty() }
        set(v) = prefs.edit().putString("adresses", v.joinToString("\n")).apply()

    /** La dernière fois qu'un poste a répondu, « JJ/MM/AAAA HH:MM ». */
    var lastSync: String
        get() = prefs.getString("derniere", "") ?: ""
        set(v) = prefs.edit().putString("derniere", v).apply()

    /** Le dossier d'échange choisi (un arbre de documents), ou rien. */
    var folder: String?
        get() = prefs.getString("dossier", null)
        set(v) = prefs.edit().putString("dossier", v).apply()

    companion object {
        const val DEFAULT_PORT = 7743
    }
}
