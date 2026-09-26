package io.github.youssefsahli.bpmcaddy

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
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
import androidx.compose.material3.Button
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.time.DayOfWeek
import java.time.LocalDate
import uniffi.bpm_caddy_mobile.Caddy
import uniffi.bpm_caddy_mobile.displayDate

private enum class Tab(val key: String, val mark: String) {
    Fiches("mobile_tab_cards", "F"),
    Agenda("mobile_tab_agenda", "A"),
    Planning("mobile_tab_planning", "P"),
    Messages("mobile_tab_messages", "M"),
    Postes("mobile_tab_posts", "O"),
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
    var tab by rememberSaveable { mutableStateOf(if (inGroup) Tab.Fiches else Tab.Postes) }
    Scaffold(
        bottomBar = {
            NavigationBar {
                Tab.entries.forEach { t ->
                    NavigationBarItem(
                        selected = tab == t,
                        onClick = { tab = t },
                        icon = { Text(t.mark, fontWeight = FontWeight.Bold) },
                        label = { Text(T(t.key), maxLines = 1) },
                    )
                }
            }
        },
        snackbarHost = {
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
        CardScreen(caddy, shown) { open = null }
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
private fun CardScreen(caddy: Caddy, id: Long, back: () -> Unit) {
    val card by produceState<uniffi.bpm_caddy_mobile.Card?>(null, id) {
        value = withContext(Dispatchers.IO) { caddy.card(id) }
    }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(12.dp)) {
        TextButton(onClick = back, contentPadding = PaddingValues(0.dp)) { Text(T("mobile_back")) }
        val c = card ?: return@Column
        Text(c.name, style = MaterialTheme.typography.titleLarge)
        val sub = listOf(c.dci, c.`class`, c.status).filter { it.isNotBlank() }.joinToString(" · ")
        if (sub.isNotEmpty()) Text(sub, style = MaterialTheme.typography.bodyMedium)
        if (c.sections.isEmpty()) Hint(T("mobile_card_empty"))
        c.sections.forEach { s ->
            Text(
                s.label,
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.padding(top = 12.dp, bottom = 2.dp),
            )
            Text(s.text, style = MaterialTheme.typography.bodyMedium)
        }
        Hint(T("mobile_card_caveat"))
    }
}

// ---------------------------------------------------------------- Semaine

/** Le lundi de la semaine de `day`. */
private fun monday(day: LocalDate): LocalDate = day.with(DayOfWeek.MONDAY)

@Composable
private fun WeekBar(start: LocalDate, move: (Long) -> Unit) {
    Row(
        Modifier.fillMaxWidth().padding(horizontal = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TextButton(onClick = { move(-1) }) { Text("‹") }
        Text(
            displayDate(start.toString()) + " – " + displayDate(start.plusDays(6).toString()),
            modifier = Modifier.weight(1f),
            style = MaterialTheme.typography.titleSmall,
        )
        TextButton(onClick = { move(0) }) { Text(T("mobile_week_now")) }
        TextButton(onClick = { move(1) }) { Text("›") }
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

@Composable
private fun AgendaScreen(state: AppState, caddy: Caddy) {
    var start by rememberSaveable { mutableStateOf(monday(LocalDate.now()).toString()) }
    val from = LocalDate.parse(start)
    val items by produceState(emptyList(), start, state.revision) {
        value = withContext(Dispatchers.IO) {
            runCatching { caddy.agenda(start, from.plusDays(6).toString()) }.getOrDefault(emptyList())
        }
    }
    Column(Modifier.fillMaxSize()) {
        WeekBar(from) { step ->
            start = if (step == 0L) monday(LocalDate.now()).toString() else from.plusWeeks(step).toString()
        }
        HorizontalDivider()
        if (items.isEmpty()) Hint(T("mobile_agenda_empty"))
        LazyColumn(Modifier.fillMaxSize()) {
            items.groupBy { it.day }.toSortedMap().forEach { (day, list) ->
                item(key = "d$day") { DayHeader(dayName(day)) }
                items(list.sortedBy { it.time }, key = { "e${it.id}-${it.day}" }) { e ->
                    Row(Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 4.dp)) {
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
        WeekBar(from) { step ->
            start = if (step == 0L) monday(LocalDate.now()).toString() else from.plusWeeks(step).toString()
        }
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
        value = withContext(Dispatchers.IO) { runCatching { caddy.conversations() }.getOrDefault(emptyList()) }
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
    LaunchedEffect(said.size) { if (said.isNotEmpty()) list.scrollToItem(said.size - 1) }
    Column(Modifier.fillMaxSize()) {
        TextButton(onClick = back) { Text(T("mobile_back")) }
        LazyColumn(Modifier.weight(1f).fillMaxWidth(), state = list) {
            items(said, key = { it.id }) { m ->
                Column(Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 4.dp)) {
                    Row {
                        Text(m.author, fontWeight = FontWeight.SemiBold, modifier = Modifier.weight(1f))
                        Text(m.sentAt, style = MaterialTheme.typography.bodySmall)
                    }
                    Text(m.body)
                    if (m.citesPatient) Hint(T("mobile_message_cites_patient"))
                }
            }
        }
        val who = state.settings.initials
        if (who.isBlank()) {
            Hint(T("mobile_messages_need_initials"))
        } else {
            Row(Modifier.fillMaxWidth().padding(8.dp), verticalAlignment = Alignment.CenterVertically) {
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
            Hint(T("mobile_sync_hint"))
        }
        HorizontalDivider(Modifier.padding(vertical = 12.dp))
        PhoneSettings(state)
    }
}

@Composable
private fun JoinForm(state: AppState) {
    var code by rememberSaveable { mutableStateOf("") }
    var name by rememberSaveable { mutableStateOf("") }
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
