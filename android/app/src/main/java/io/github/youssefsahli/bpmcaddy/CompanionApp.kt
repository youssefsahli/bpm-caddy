package io.github.youssefsahli.bpmcaddy

import android.content.Intent
import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.Email
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Shapes
import androidx.compose.material3.Snackbar
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import java.time.DayOfWeek
import java.time.LocalDate
import uniffi.bpm_caddy_mobile.Caddy
import uniffi.bpm_caddy_mobile.displayDate
import uniffi.bpm_caddy_mobile.eventCategories

private enum class Tab(val key: String, val icon: ImageVector) {
    Fiches("mobile_tab_cards", Icons.Filled.Search),
    Agenda("mobile_tab_agenda", Icons.Filled.DateRange),
    Planning("mobile_tab_planning", Icons.Filled.Person),
    Messages("mobile_tab_messages", Icons.Filled.Email),
    Postes("mobile_tab_posts", Icons.Filled.Settings),
}

/** Les coins carrés du bureau (Motif), adoucis d'un rien pour le doigt. */
private val SquareShapes = Shapes(
    extraSmall = RoundedCornerShape(2.dp),
    small = RoundedCornerShape(2.dp),
    medium = RoundedCornerShape(3.dp),
    large = RoundedCornerShape(4.dp),
    extraLarge = RoundedCornerShape(4.dp),
)

@Composable
fun CompanionApp(state: AppState, unlock: () -> Unit) {
    val dark = isSystemInDarkTheme()
    val scheme = if (dark) {
        darkColorScheme(primary = Color(0xFF8FB3D9), secondary = Color(0xFFB9C7D6))
    } else {
        lightColorScheme(primary = Color(0xFF2D5E8C), secondary = Color(0xFF4F6478))
    }
    MaterialTheme(colorScheme = scheme, shapes = SquareShapes) {
        val caddy = state.caddy
        if (caddy == null) {
            Locked(state, unlock)
        } else {
            Opened(state, caddy)
        }
    }
}

@Composable
private fun Locked(state: AppState, unlock: () -> Unit) {
    LaunchedEffect(Unit) { unlock() }
    Scaffold { pad ->
        Column(
            Modifier.fillMaxSize().padding(pad).padding(24.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text("BPM-Caddy", style = MaterialTheme.typography.headlineSmall)
            Spacer(Modifier.padding(8.dp))
            Text(T("mobile_locked"), style = MaterialTheme.typography.bodyMedium)
            Spacer(Modifier.padding(8.dp))
            Button(onClick = unlock) { Text(T("mobile_unlock")) }
            state.note?.let { Text(it, Modifier.padding(top = 16.dp)) }
        }
    }
}

@Composable
private fun Opened(state: AppState, caddy: Caddy) {
    val inGroup = state.status?.inGroup == true
    // Chosen by a tap, or else by what the phone is: the cards once it
    // has joined, the joining form before. Read at each composition, so
    // the status arriving after the first frame still decides.
    var chosen by rememberSaveable { mutableStateOf<Tab?>(null) }
    val tab = chosen ?: if (inGroup) Tab.Fiches else Tab.Postes
    Scaffold(
        bottomBar = {
            NavigationBar {
                Tab.entries.forEach { t ->
                    NavigationBarItem(
                        selected = tab == t,
                        onClick = { chosen = t },
                        icon = { Icon(t.icon, contentDescription = null) },
                        label = { Text(T(t.key), maxLines = 1) },
                    )
                }
            }
        },
        snackbarHost = {
            // Lu, puis parti : une phrase longue reste le temps de la lire.
            LaunchedEffect(state.note) {
                val shown = state.note ?: return@LaunchedEffect
                delay(4000L + 60L * shown.length)
                if (state.note == shown) state.dismiss()
            }
            state.note?.let {
                Snackbar(
                    modifier = Modifier.padding(8.dp),
                    action = { TextButton(onClick = { state.dismiss() }) { Text(T("mobile_ok")) } },
                ) { Text(it) }
            }
        },
    ) { pad ->
        // The bottom bar's inset is already in `pad`: the keyboard only
        // adds what it covers beyond it.
        Column(Modifier.fillMaxSize().padding(pad).consumeWindowInsets(pad).imePadding()) {
            if (state.busy) LinearProgressIndicator(Modifier.fillMaxWidth())
            when (tab) {
                Tab.Fiches -> CardsScreen(state, caddy)
                Tab.Agenda -> AgendaScreen(state, caddy)
                Tab.Planning -> PlanningScreen(state, caddy)
                Tab.Messages -> MessagesScreen(state, caddy)
                Tab.Postes -> PostsScreen(state)
            }
        }
    }
}

// ---------------------------------------------------------------- Fiches

@Composable
private fun CardsScreen(state: AppState, caddy: Caddy) {
    var query by rememberSaveable { mutableStateOf("") }
    var open by rememberSaveable { mutableStateOf<Long?>(null) }
    val shown = open
    if (shown != null) {
        BackHandler { open = null }
        CardScreen(state, caddy, shown) { open = null }
        return
    }
    val hits by produceState(emptyList(), query, state.revision) {
        value = withContext(Dispatchers.IO) { caddy.searchCards(query, 200u) }
    }
    Column(Modifier.fillMaxSize()) {
        OutlinedTextField(
            value = query,
            onValueChange = { query = it },
            label = { Text(T("mobile_cards_search")) },
            singleLine = true,
            modifier = Modifier.fillMaxWidth().padding(8.dp),
        )
        if (hits.isEmpty()) {
            Hint(T(if (query.isBlank()) "mobile_cards_none" else "mobile_cards_no_hit"))
        }
        LazyColumn(Modifier.fillMaxSize()) {
            items(hits, key = { it.id }) { h ->
                Column(
                    Modifier.fillMaxWidth().clickable { open = h.id }
                        .padding(horizontal = 12.dp, vertical = 6.dp),
                ) {
                    Text(h.name, fontWeight = FontWeight.SemiBold)
                    val sub = listOf(h.dci, h.`class`).filter { it.isNotBlank() }.joinToString(" · ")
                    if (sub.isNotEmpty()) Text(sub, style = MaterialTheme.typography.bodySmall)
                }
                HorizontalDivider()
            }
        }
    }
}

@Composable
private fun CardScreen(state: AppState, caddy: Caddy, id: Long, back: () -> Unit) {
    val card by produceState<uniffi.bpm_caddy_mobile.Card?>(null, id, state.revision) {
        value = withContext(Dispatchers.IO) { caddy.card(id) }
    }
    var editing by remember { mutableStateOf<uniffi.bpm_caddy_mobile.Section?>(null) }
    editing?.let { s ->
        var text by remember(s.key) { mutableStateOf(s.text) }
        AlertDialog(
            onDismissRequest = { editing = null },
            title = { Text(s.label) },
            text = {
                OutlinedTextField(
                    text, { text = it }, Modifier.fillMaxWidth(), minLines = 6,
                    keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences),
                )
            },
            confirmButton = {
                TextButton(
                    onClick = { state.editSection(id, s.key, s.text, text) { editing = null } },
                    enabled = state.settings.initials.isNotBlank(),
                ) { Text(T("mobile_save")) }
            },
            dismissButton = { TextButton(onClick = { editing = null }) { Text(T("mobile_cancel")) } },
        )
    }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(12.dp)) {
        TextButton(onClick = back, contentPadding = PaddingValues(0.dp)) { Text(T("mobile_back")) }
        val c = card ?: return@Column
        Text(c.name, style = MaterialTheme.typography.titleLarge)
        val sub = listOf(c.dci, c.`class`, c.status).filter { it.isNotBlank() }.joinToString(" · ")
        if (sub.isNotEmpty()) Text(sub, style = MaterialTheme.typography.bodyMedium)
        if (c.sections.isEmpty()) Hint(T("mobile_card_empty"))
        if (state.settings.initials.isBlank()) Hint(T("mobile_card_need_initials"))
        c.sections.forEach { s ->
            Row(Modifier.padding(top = 12.dp, bottom = 2.dp), verticalAlignment = Alignment.CenterVertically) {
                Text(
                    s.label,
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.weight(1f),
                )
                if (state.settings.initials.isNotBlank()) {
                    TextButton(onClick = { editing = s }, contentPadding = PaddingValues(0.dp)) {
                        Text(T("mobile_card_edit"))
                    }
                }
            }
            Text(s.text, style = MaterialTheme.typography.bodyMedium)
        }
        Hint(T("mobile_card_caveat"))
    }
}

// ---------------------------------------------------------------- Semaine

/** Le lundi de la semaine de `day`. */
private fun monday(day: LocalDate): LocalDate = day.with(DayOfWeek.MONDAY)

/**
 * La semaine montrée : ‹ et › pour changer, les dates pour revenir à
 * celle-ci, et, à droite, le geste de l'écran (« Ajouter » pour l'agenda).
 */
@Composable
private fun WeekBar(start: LocalDate, move: (Long) -> Unit, action: @Composable () -> Unit = {}) {
    Row(
        Modifier.fillMaxWidth().padding(horizontal = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TextButton(onClick = { move(-1) }) { Text("‹") }
        Column(Modifier.weight(1f).clickable { move(0) }) {
            Text(
                displayDate(start.toString()) + " – " + displayDate(start.plusDays(6).toString()),
                style = MaterialTheme.typography.titleSmall,
            )
            if (start != monday(LocalDate.now())) {
                Text(T("mobile_week_now"), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
            }
        }
        TextButton(onClick = { move(1) }) { Text("›") }
        action()
    }
}

/** Les jours, lundi d'abord — écrits en entier pour que le test des clés les lise. */
private val DAYS = listOf(
    "mobile_day_1", "mobile_day_2", "mobile_day_3", "mobile_day_4",
    "mobile_day_5", "mobile_day_6", "mobile_day_7",
)

private fun dayName(iso: String): String {
    val d = LocalDate.parse(iso)
    return T(DAYS[d.dayOfWeek.value - 1]) + " " + displayDate(iso)
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun AgendaScreen(state: AppState, caddy: Caddy) {
    var start by rememberSaveable { mutableStateOf(monday(LocalDate.now()).toString()) }
    val from = LocalDate.parse(start)
    val items by produceState(emptyList(), start, state.revision) {
        value = withContext(Dispatchers.IO) {
            runCatching { caddy.agenda(start, from.plusDays(6).toString()) }.getOrDefault(emptyList())
        }
    }
    var adding by rememberSaveable { mutableStateOf(false) }
    var removing by remember { mutableStateOf<uniffi.bpm_caddy_mobile.AgendaItem?>(null) }
    if (adding) {
        EventForm(state) { adding = false }
    }
    removing?.let { e ->
        AlertDialog(
            onDismissRequest = { removing = null },
            title = { Text(T("mobile_agenda_delete")) },
            text = { Text(e.title + " · " + dayName(e.day)) },
            confirmButton = {
                TextButton(onClick = {
                    state.deleteEvent(e.id, e.title)
                    removing = null
                }) { Text(T("mobile_delete")) }
            },
            dismissButton = { TextButton(onClick = { removing = null }) { Text(T("mobile_cancel")) } },
        )
    }
    Column(Modifier.fillMaxSize()) {
        WeekBar(from, { step ->
            start = if (step == 0L) monday(LocalDate.now()).toString() else from.plusWeeks(step).toString()
        }) {
            OutlinedButton(onClick = { adding = true }, contentPadding = PaddingValues(horizontal = 12.dp)) {
                Text(T("mobile_agenda_add"))
            }
        }
        HorizontalDivider()
        if (items.isEmpty()) Hint(T("mobile_agenda_empty"))
        LazyColumn(Modifier.fillMaxSize()) {
            items.groupBy { it.day }.toSortedMap().forEach { (day, list) ->
                item(key = "d$day") { DayHeader(dayName(day)) }
                items(list.sortedBy { it.time }, key = { "e${it.id}-${it.day}" }) { e ->
                    Row(
                        Modifier.fillMaxWidth()
                            .combinedClickable(onClick = {}, onLongClick = { removing = e })
                            .padding(horizontal = 12.dp, vertical = 4.dp),
                    ) {
                        val hours = listOf(e.time, e.endTime).filter { it.isNotBlank() }.joinToString("–")
                        Text(hours, Modifier.width(96.dp), style = MaterialTheme.typography.bodyMedium)
                        Column(Modifier.weight(1f)) {
                            Text(e.title)
                            Text(e.category, style = MaterialTheme.typography.bodySmall)
                        }
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun EventForm(state: AppState, close: () -> Unit) {
    var day by rememberSaveable { mutableStateOf(displayDate(today())) }
    var from by rememberSaveable { mutableStateOf("") }
    var to by rememberSaveable { mutableStateOf("") }
    var title by rememberSaveable { mutableStateOf("") }
    val kinds = remember { eventCategories() }
    var kind by rememberSaveable { mutableStateOf(kinds.lastOrNull()?.key ?: "") }
    AlertDialog(
        onDismissRequest = close,
        title = { Text(T("mobile_agenda_new")) },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                OutlinedTextField(day, { day = it }, label = { Text(T("mobile_agenda_day")) }, singleLine = true)
                Row {
                    OutlinedTextField(from, { from = it }, Modifier.weight(1f), label = { Text(T("mobile_agenda_from")) }, singleLine = true)
                    Spacer(Modifier.width(8.dp))
                    OutlinedTextField(to, { to = it }, Modifier.weight(1f), label = { Text(T("mobile_agenda_to")) }, singleLine = true)
                }
                OutlinedTextField(
                    title, { title = it }, label = { Text(T("mobile_agenda_title")) }, singleLine = true,
                    keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences),
                )
                FlowRow(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    kinds.forEach { c ->
                        FilterChip(selected = kind == c.key, onClick = { kind = c.key }, label = { Text(c.label) })
                    }
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = { state.addEvent(day, from, to, title, kind) { ok -> if (ok) close() } },
                enabled = title.isNotBlank(),
            ) { Text(T("mobile_save")) }
        },
        dismissButton = { TextButton(onClick = close) { Text(T("mobile_cancel")) } },
    )
}

@Composable
private fun PlanningScreen(state: AppState, caddy: Caddy) {
    var start by rememberSaveable { mutableStateOf(monday(LocalDate.now()).toString()) }
    val from = LocalDate.parse(start)
    val shifts by produceState(emptyList(), start, state.revision) {
        value = withContext(Dispatchers.IO) {
            runCatching { caddy.planning(start, from.plusDays(6).toString()) }.getOrDefault(emptyList())
        }
    }
    Column(Modifier.fillMaxSize()) {
        WeekBar(from, move = { step ->
            start = if (step == 0L) monday(LocalDate.now()).toString() else from.plusWeeks(step).toString()
        })
        HorizontalDivider()
        if (shifts.isEmpty()) Hint(T("mobile_planning_empty"))
        LazyColumn(Modifier.fillMaxSize()) {
            shifts.groupBy { it.day }.toSortedMap().forEach { (day, list) ->
                item(key = "d$day") { DayHeader(dayName(day)) }
                items(list.sortedWith(compareBy({ it.startTime }, { it.operator }))) { s ->
                    Row(Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 3.dp)) {
                        Text(s.operator, Modifier.width(56.dp), fontWeight = FontWeight.SemiBold)
                        val hours = listOf(s.startTime, s.endTime).filter { it.isNotBlank() }.joinToString("–")
                        Text(hours, Modifier.width(110.dp))
                        Text(
                            listOf(s.kind, s.note).filter { it.isNotBlank() }.joinToString(" · "),
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------- Messages

@Composable
private fun MessagesScreen(state: AppState, caddy: Caddy) {
    var open by rememberSaveable { mutableStateOf<Long?>(null) }
    val shown = open
    if (shown != null) {
        BackHandler { open = null }
        ThreadScreen(state, caddy, shown) { open = null }
        return
    }
    val talks by produceState(emptyList(), state.revision) {
        value = withContext(Dispatchers.IO) {
            runCatching { caddy.conversations(state.settings.initials) }.getOrDefault(emptyList())
        }
    }
    Column(Modifier.fillMaxSize()) {
        Row(Modifier.fillMaxWidth().padding(8.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(T("mobile_tab_messages"), Modifier.weight(1f), style = MaterialTheme.typography.titleMedium)
            OutlinedButton(onClick = { state.sync() }, enabled = !state.busy) { Text(T("mobile_sync")) }
        }
        if (talks.isEmpty()) Hint(T("mobile_messages_empty"))
        LazyColumn(Modifier.fillMaxSize()) {
            items(talks, key = { it.id }) { t ->
                Column(
                    Modifier.fillMaxWidth().clickable { open = t.id }
                        .padding(horizontal = 12.dp, vertical = 6.dp),
                ) {
                    Row {
                        Text(t.title.ifBlank { T("mobile_messages_team") }, Modifier.weight(1f), fontWeight = FontWeight.SemiBold)
                        if (t.unread > 0u) {
                            Text(
                                T("mobile_messages_unread").replace("{}", t.unread.toString()),
                                color = MaterialTheme.colorScheme.primary,
                                fontWeight = FontWeight.Bold,
                                modifier = Modifier.padding(end = 8.dp),
                            )
                        }
                        Text(t.lastAt, style = MaterialTheme.typography.bodySmall)
                    }
                    if (t.last.isNotBlank()) Text(t.last, maxLines = 1, style = MaterialTheme.typography.bodySmall)
                }
                HorizontalDivider()
            }
        }
    }
}

@Composable
private fun ThreadScreen(state: AppState, caddy: Caddy, id: Long, back: () -> Unit) {
    var tick by remember { mutableStateOf(0) }
    val said by produceState(emptyList(), id, state.revision, tick) {
        value = withContext(Dispatchers.IO) { runCatching { caddy.messages(id) }.getOrDefault(emptyList()) }
    }
    var draft by rememberSaveable { mutableStateOf("") }
    val list = rememberLazyListState()
    LaunchedEffect(said.size) {
        if (said.isNotEmpty()) {
            list.scrollToItem(said.size - 1)
            val marked = withContext(Dispatchers.IO) {
                runCatching { caddy.markRead(id, state.settings.initials) }.isSuccess
            }
            if (marked) state.touched()
        }
    }
    Column(Modifier.fillMaxSize()) {
        TextButton(onClick = back) { Text(T("mobile_back")) }
        LazyColumn(Modifier.weight(1f).fillMaxWidth(), state = list) {
            items(said, key = { it.id }) { m ->
                Column(Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 4.dp)) {
                    Row {
                        Text(m.author, fontWeight = FontWeight.SemiBold, modifier = Modifier.weight(1f))
                        Text(m.sentAt, style = MaterialTheme.typography.bodySmall)
                    }
                    if (m.body.isNotBlank()) Text(m.body)
                    m.files.forEach { f ->
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text(
                                f.name + " · " + if (f.size < 1024u) {
                                    T("mobile_file_bytes").replace("{}", f.size.toString())
                                } else {
                                    T("mobile_file_kib").replace("{}", (f.size / 1024u).toString())
                                },
                                Modifier.weight(1f),
                                style = MaterialTheme.typography.bodySmall,
                            )
                            if (f.complete) {
                                TextButton(onClick = { state.openFile(id, f.uid) }) { Text(T("mobile_file_open")) }
                            } else {
                                Text(T("mobile_file_pending"), style = MaterialTheme.typography.bodySmall)
                            }
                        }
                    }
                    if (m.citesPatient) Hint(T("mobile_message_cites_patient"))
                }
            }
        }
        val who = state.settings.initials
        if (who.isBlank()) {
            Hint(T("mobile_messages_need_initials"))
        } else {
            val attach = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
                if (uri != null) {
                    val body = draft
                    draft = ""
                    state.sendFile(id, body, uri) { tick++ }
                }
            }
            Row(Modifier.fillMaxWidth().padding(8.dp), verticalAlignment = Alignment.CenterVertically) {
                TextButton(onClick = { attach.launch(arrayOf("*/*")) }, contentPadding = PaddingValues(4.dp)) {
                    Text(T("mobile_file_attach"))
                }
                OutlinedTextField(
                    value = draft,
                    onValueChange = { draft = it },
                    modifier = Modifier.weight(1f),
                    placeholder = { Text(T("mobile_message_draft")) },
                    keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences),
                )
                Button(
                    onClick = {
                        val body = draft
                        draft = ""
                        state.send(id, body) { tick++ }
                    },
                    enabled = draft.isNotBlank(),
                    modifier = Modifier.padding(start = 8.dp),
                ) { Text(T("mobile_send")) }
            }
        }
    }
}

// ---------------------------------------------------------------- Postes

@Composable
private fun PostsScreen(state: AppState) {
    val s = state.status
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(12.dp)) {
        if (s == null || !s.inGroup) {
            JoinForm(state)
        } else {
            Text(T("mobile_posts_group"), style = MaterialTheme.typography.titleMedium)
            Text(s.group, style = MaterialTheme.typography.bodyMedium)
            Text(T("mobile_posts_me") + " " + s.device, style = MaterialTheme.typography.bodySmall)
            Spacer(Modifier.padding(4.dp))
            s.posts.forEach { p ->
                Row(Modifier.fillMaxWidth().padding(vertical = 2.dp)) {
                    Text("n° " + (p.number + 1), Modifier.width(56.dp))
                    Text(
                        p.name.ifBlank { T("mobile_posts_unnamed") } +
                            (if (p.companion) " · " + T("mobile_posts_phone") else "") +
                            (if (p.me) " · " + T("mobile_posts_this_one") else ""),
                    )
                }
            }
            Text(
                T("mobile_posts_cards").replace("{}", s.cards.toString()),
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.padding(top = 4.dp),
            )
            Spacer(Modifier.padding(6.dp))
            Button(onClick = { state.sync() }, enabled = !state.busy) { Text(T("mobile_sync")) }
            val last = state.settings.lastSync
            Hint(if (last.isBlank()) T("mobile_sync_never") else T("mobile_sync_last").replace("{}", last))
            Hint(T("mobile_sync_hint"))
        }
        HorizontalDivider(Modifier.padding(vertical = 12.dp))
        PhoneSettings(state)
        HorizontalDivider(Modifier.padding(vertical = 12.dp))
        WipeButton(state)
    }
}

@Composable
private fun WipeButton(state: AppState) {
    var asking by remember { mutableStateOf(false) }
    Hint(T("mobile_wipe_hint"))
    OutlinedButton(onClick = { asking = true }) { Text(T("mobile_wipe")) }
    if (asking) {
        AlertDialog(
            onDismissRequest = { asking = false },
            title = { Text(T("mobile_wipe_confirm")) },
            text = { Text(T("mobile_wipe_hint")) },
            confirmButton = {
                TextButton(onClick = {
                    asking = false
                    state.wipe()
                }) { Text(T("mobile_wipe")) }
            },
            dismissButton = { TextButton(onClick = { asking = false }) { Text(T("mobile_cancel")) } },
        )
    }
}

@Composable
private fun JoinForm(state: AppState) {
    var code by rememberSaveable { mutableStateOf("") }
    // Le modèle du téléphone, à défaut d'un nom : un nom dans la liste des
    // postes vaut mieux que « Sans nom ».
    var name by rememberSaveable { mutableStateOf(android.os.Build.MODEL ?: "") }
    Text(T("mobile_join_title"), style = MaterialTheme.typography.titleMedium)
    Hint(T("mobile_join_hint"))
    OutlinedTextField(
        value = code,
        onValueChange = { code = it },
        label = { Text(T("mobile_join_code")) },
        singleLine = true,
        keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Characters),
        modifier = Modifier.fillMaxWidth(),
    )
    OutlinedTextField(
        value = name,
        onValueChange = { name = it },
        label = { Text(T("mobile_join_name")) },
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
    )
    Button(
        onClick = { state.join(code, name) },
        enabled = !state.busy && code.isNotBlank(),
        modifier = Modifier.padding(top = 8.dp),
    ) { Text(T("mobile_join")) }
}

@Composable
private fun PhoneSettings(state: AppState) {
    var initials by rememberSaveable { mutableStateOf(state.settings.initials) }
    var port by rememberSaveable { mutableStateOf(state.settings.port.toString()) }
    var addresses by rememberSaveable { mutableStateOf(state.settings.addresses.joinToString("\n")) }
    Text(T("mobile_settings"), style = MaterialTheme.typography.titleMedium)
    OutlinedTextField(
        value = initials,
        onValueChange = { initials = it.take(4) },
        label = { Text(T("mobile_settings_initials")) },
        singleLine = true,
        keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Characters),
        modifier = Modifier.fillMaxWidth(),
    )
    OutlinedTextField(
        value = port,
        onValueChange = { port = it.filter(Char::isDigit).take(5) },
        label = { Text(T("mobile_settings_port")) },
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
        modifier = Modifier.fillMaxWidth(),
    )
    OutlinedTextField(
        value = addresses,
        onValueChange = { addresses = it },
        label = { Text(T("mobile_settings_addresses")) },
        modifier = Modifier.fillMaxWidth(),
        minLines = 2,
    )
    FolderSetting(state)
    Button(
        onClick = {
            state.settings.initials = initials
            state.settings.port = port.toIntOrNull()?.takeIf { it in 1..65535 } ?: Settings.DEFAULT_PORT
            state.settings.addresses = addresses.split('\n')
            state.notice(T("mobile_settings_saved"))
        },
        modifier = Modifier.padding(top = 8.dp),
    ) { Text(T("mobile_save")) }
}

@Composable
private fun FolderSetting(state: AppState) {
    val context = LocalContext.current
    var folder by remember { mutableStateOf(state.settings.folder) }
    val pick = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
        if (uri != null) {
            context.contentResolver.takePersistableUriPermission(
                uri,
                Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
            )
            state.settings.folder = uri.toString()
            folder = uri.toString()
        }
    }
    Text(T("mobile_folder"), style = MaterialTheme.typography.titleSmall, modifier = Modifier.padding(top = 8.dp))
    Hint(T("mobile_folder_hint"))
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text(
            folder?.let { Uri.decode(it).substringAfterLast(':') } ?: T("mobile_folder_none"),
            Modifier.weight(1f),
            maxLines = 1,
        )
        TextButton(onClick = { pick.launch(null) }) { Text(T("mobile_folder_choose")) }
        if (folder != null) {
            TextButton(onClick = {
                state.settings.folder = null
                folder = null
            }) { Text(T("mobile_folder_forget")) }
        }
    }
}

// ---------------------------------------------------------------- Petits

@Composable
private fun DayHeader(text: String) {
    Box(
        Modifier.fillMaxWidth().padding(top = 8.dp)
            .padding(horizontal = 12.dp, vertical = 4.dp),
    ) {
        Text(text, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)
    }
}

@Composable
private fun Hint(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
    )
}
