package io.github.youssefsahli.bpmcaddy

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.bpm_caddy_mobile.Caddy
import uniffi.bpm_caddy_mobile.CaddyException
import uniffi.bpm_caddy_mobile.Status

/**
 * Ce que l'écran sait : la base ouverte ou non, où en est le téléphone,
 * la dernière phrase à montrer. Tout appel à la bibliothèque passe ici,
 * sur un fil d'arrière-plan — une synchronisation parle au réseau.
 */
class AppState(private val context: Context) {
    val settings = Settings(context)
    private val vault = Vault(context)
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    var caddy: Caddy? by mutableStateOf(null)
        private set
    var status: Status? by mutableStateOf(null)
        private set
    var busy by mutableStateOf(false)
        private set
    var note: String? by mutableStateOf(null)
        private set

    /** Monte à chaque synchronisation : les listes se relisent. */
    var revision by mutableIntStateOf(0)
        private set

    fun notice(text: String) {
        note = text
    }

    fun dismiss() {
        note = null
    }

    /** Ouvrir la base, juste après l'authentification. */
    fun open() {
        scope.launch {
            val opened = withContext(Dispatchers.IO) {
                runCatching {
                    val phrase = vault.passphrase()
                    Caddy.open(context.filesDir.absolutePath, phrase)
                }
            }
            val c = opened.getOrElse {
                note = said(it)
                return@launch
            }
            caddy = c
            status = withContext(Dispatchers.IO) { runCatching { c.status() }.getOrNull() }
            revision++
            // Ouvert, il va chercher ce qui a changé.
            if (status?.inGroup == true) sync()
        }
    }

    fun refresh() {
        val c = caddy ?: return
        scope.launch {
            status = withContext(Dispatchers.IO) { runCatching { c.status() }.getOrNull() }
            revision++
        }
    }

    fun join(code: String, name: String) {
        val c = caddy ?: return
        work {
            withMulticast(context) {
                c.join(code.trim(), name.trim(), today(), settings.port.toUShort())
            }
        }
    }

    fun sync() {
        val c = caddy ?: return
        work {
            withMulticast(context) {
                c.sync(today(), settings.port.toUShort(), settings.addresses).said
            }
        }
    }

    fun send(conversation: Long, body: String, then: () -> Unit) {
        val c = caddy ?: return
        scope.launch {
            val done = withContext(Dispatchers.IO) {
                runCatching { c.sendMessage(conversation, settings.initials, body) }
            }
            done.onFailure { note = said(it) }
            then()
            // Part tout de suite si un poste est là.
            sync()
        }
    }

    /** Ajouter à l'agenda ; `then(true)` quand c'est écrit. */
    fun addEvent(day: String, from: String, to: String, title: String, category: String, then: (Boolean) -> Unit) {
        val c = caddy ?: return
        scope.launch {
            val done = withContext(Dispatchers.IO) {
                runCatching { c.addEvent(day, from, to, title, category, today()) }
            }
            done.onFailure { note = said(it) }
            then(done.isSuccess)
            if (done.isSuccess) {
                revision++
                sync()
            }
        }
    }

    /** Récrire une section de fiche ; `then(true)` quand c'est écrit. */
    fun editSection(id: Long, key: String, shown: String, text: String, then: (Boolean) -> Unit) {
        val c = caddy ?: return
        scope.launch {
            val done = withContext(Dispatchers.IO) {
                runCatching { c.editCardSection(id, key, shown, text, settings.initials) }
            }
            done.onFailure { note = said(it) }
            if (done.getOrNull() == false) note = T("mobile_card_stale")
            revision++
            then(done.getOrNull() == true)
            if (done.getOrNull() == true) sync()
        }
    }

    /** Retirer une entrée, si elle dit encore ce que l'écran montrait. */
    fun deleteEvent(id: Long, shownTitle: String) {
        val c = caddy ?: return
        scope.launch {
            val done = withContext(Dispatchers.IO) { runCatching { c.deleteEvent(id, shownTitle) } }
            done.onFailure { note = said(it) }
            if (done.getOrNull() == false) note = T("mobile_agenda_stale")
            revision++
            if (done.getOrNull() == true) sync()
        }
    }

    private fun work(block: () -> String) {
        if (busy) return
        busy = true
        scope.launch {
            val done = withContext(Dispatchers.IO) { runCatching(block) }
            busy = false
            note = done.fold({ it }, { said(it) })
            refresh()
        }
    }

    private fun said(e: Throwable): String = when (e) {
        is CaddyException.Failed -> e.reason
        else -> e.message ?: e.toString()
    }
}
