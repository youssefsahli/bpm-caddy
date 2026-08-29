# BPM-Caddy — notes for Claude Code

Clinical pharmacy desktop app (Rust + egui), UI in French, proprietary
license with free public releases. Spec: `docs/SPECIFICATIONS.txt`.

## Workspace

- root `bpm-caddy` — the app: `src/app.rs` (UI/state), `src/db.rs`
  (SQLCipher storage), `src/fuzzy.rs` (search), `src/pdf.rs` (Typst),
  `src/config.rs` (config.toml), `src/vaccines.rs` (calendrier vaccinal
  rules and the traveller's country table — static, pure, tested),
  `src/bulletin.rs` (fills the official Assurance Maladie bulletins
  d'adhésion in `assets/bulletins/`), `src/ordonnance.rs` (what a
  positive TROD allows, and the choices behind the ordonnance),
  `src/codex.rs` (reading a preparation's formula and rescaling it —
  pure and tested; the preparations themselves live in the base),
  `src/entretien.rs` (what each thematic covers, printed on the fiche),
  `src/biology.rs` (the analytes, their usual intervals, and the rules
  that read a value against the patient's treatments — static, pure,
  tested), `src/revue.rs` (what a set of treatments says about itself:
  doublons, associations, cascades — same shape, same discipline),
  `src/conciliation.rs` (the file's ordonnance against the one a patient
  brings back from hospital: reads a pasted list, matches each line to a
  fiche, and says what was stopped, changed, added or replaced — pure,
  tested, no catalogue of its own),
  `src/surveillance.rs` (what a treatment asks to have measured and how
  often, read against the dates already in the file — the other half of
  `biology.rs`: that one reads the values that are there, this one names
  the ones that are not). The
  dispositifs médicaux have no module: they are fiches in the base
  (`STARTER_DISPOSITIFS` in `src/db.rs`), like the codex.
  `src/location.rs` (what a rental of material owes and when its
  ordonnance runs out — pure, tested, no internal clock; the forfaits
  live in `[locations]` of config.toml and ship empty),
  `src/insulin.rs` (each insulin's action profile as a curve, and the
  500 / 1800 / titration rules — static, pure, tested),
  `src/release.rs` (what version this is, and — only on a button press,
  from Options › À propos — what GitHub says the newest release is; the
  **only** network request in the application, everything else hands a
  URL to the browser),
  `src/vitale.rs` (finding the beneficiaries on a carte Vitale — the NIR
  proves itself by its control key, so nothing is read at an offset
  anybody guessed; pure and tested apart from the one transmission
  function), `src/winscard.rs` (the PC/SC library, opened **by name at
  the moment a card is asked for and never linked against**: a binary
  linked to `libpcsclite` does not start at all on a post that has none,
  and most posts have none — which is also why no system package is
  needed to build this, in CI or on the release runners either)
- `launcher/` — `bpm-caddy-launcher`, auto-updates from GitHub Releases
- `motif/` — X/Motif theme for egui (palette, bevels, custom widgets)

Always build/lint with `--workspace`: plain `cargo build` only builds the
root package. CI enforces `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings` (the tests are
linted too), `cargo test --workspace`, and `./scripts/coverage.sh`.

`scripts/coverage.sh` holds two floors that only ever move up: the
**logic modules** (everything but `app.rs`, `main.rs`, `motif` and the
launcher) and the workspace as a whole. The workspace figure is low and
will stay low: `src/app.rs` is ~15 000 lines of egui layout, more than
half the repo, and a view cannot be covered without a UI harness —
`egui_kittest` needs egui ≥ 0.30 and the project is on 0.29. Until that
upgrade is decided, the number to defend is the logic one, and
`scripts/smoke.sh` is what holds the interface.

## Conventions

- User-facing strings are French; code and comments are English. UI
  strings live in `assets/strings.fr.toml`, accessed via
  `strings::tr/trf/trn` (user override: `strings.toml` next to
  `config.toml`) — never hardcode a new UI string.
- Dates: stored ISO `YYYY-MM-DD`, displayed `JJ/MM/AAAA`
  (`db::parse_french_date` / `db::format_french_date`). Input accepts
  compact shorthand (`230826`, `2308`); `db::YearHint` picks how
  two-digit years expand (birth dates → past, RDV → 20xx).
- Schema changes go in `SCHEMA` **and** as an idempotent `ALTER TABLE` in
  `MIGRATIONS` (`src/db.rs`). This is now enforced rather than asked
  for: `tests/fixtures/schema-0.109.0.sql` is a photograph of the schema
  as that version shipped it, and
  `a_base_from_an_older_version_still_answers_every_query` creates a
  base from it, opens it, and runs every read the application makes.
  Forget the `ALTER` and every other test still passes — they all run on
  bases *this* version created, where `SCHEMA` put the column there
  anyway — and that one fails with « no such column ».
- Keep the Motif look: square corners, `motif::bevel` raised for
  buttons/panels, sunken for inputs/troughs; charts are painted by hand,
  no plotting library — `motif::chart` has bars, hbars, stacked,
  sparkline, meter, pips, heat strip and legend.
- **The application ships no font.** It draws with egui's own faces, and
  the proportional one has no arrows (U+2192 and friends) — the
  monospace one does. So an arrow may appear in a key chip and never in
  a sentence, and three strings were hollow boxes on screen for months
  because the chip beside them was right. Two tests in `src/strings.rs`
  pass every shipped character through the face that will draw it.
- **Colour comes from the theme, never from a literal.** `motif::bg()`,
  `text_dim()`, `accent()`… are functions over `motif::THEMES` (six
  palettes, `[ui] theme`); a hard-coded `Color32::from_rgb` in the
  chrome is a colour that will look wrong on five of the six. Chart
  *data* colours (`chart::series_color`) are the exception and stay
  fixed, except the first, which follows the accent. A new palette must
  pass `every_palette_can_be_read`.
- **Layout is carved, not stacked.** A view computes rectangles with
  `motif::split_rows` / `split_columns` and fills them with
  `motif::panel` / `well` / `inside`; it does not centre a fixed-width
  column and let content run down the page. Measure against
  `motif::visible_rect(ui)`, never `available_rect_before_wrap` alone —
  a dock that grew past the width it reserved leaves the central view
  laid out wider than it is visible.
- A band whose height depends on its content (wrapped buttons, filters)
  measures it with `Self::wrapped_rows` and is **capped**, scrolling past
  its share rather than crowding out the panes under it. Every layout
  must survive 1024x700 with both docks open — `scripts/smoke.sh` opens
  every view **twice**, at 1400x900 and at 1024x700 with
  `text_scale = 1.25`, and a screenshot at that size is the eye check
  the panics test cannot do.
- A control that must stay visible under a widget that grows (a button
  under a text box) gets its **own carved row**, taken off the bottom
  with `split_rows` before the widget is drawn — not a height reserved
  in the flow above it. `add_sized` sizes the *text*, and a
  `TextEdit::multiline` adds its own frame margin on top, so a reserve
  computed from `available_height` is always a few pixels short and the
  button ends up half painted. `Self::button_height` is the button's
  real height; `interact_size.y` is not.
- `allocate_new_ui` only sets a max rect and egui paints through it: use
  `motif::inside` when content must not escape its frame. It also
  reserves **no space**, so a `ScrollArea` around it never learns the
  content is wider than the viewport and offers no bar.
- **Measure, never guess a threshold.** Every layout bug found in the
  0.94–0.102 pass was a pixel constant standing in for a measurement:
  « narrower than 620 px » put the file's buttons across the patient's
  name at 645; a 360 px reserve for the agenda's title field had
  forgotten the width of the button after it; a 170 px floor under the
  notes journal was three of its own lines at text scale 1,25. Use
  `Self::button_width`, `Self::wrapped_rows(_of)`,
  `ui.text_style_height(&TextStyle::Body)` — and express floors in
  **lines**, so `[ui] text_scale` costs nothing.
- **When two panes cannot both fit, the one you type into wins.** A
  table missing a row still reads and scrolls; a form whose second row
  of fields is cut cannot be used. The same rule settled the carnet
  (form over table), the journal (add row over well) and the acts tab
  (table over journal — there the table *is* the subject).
- `f32::clamp` **panics** when min > max, and computed floors do cross
  computed caps on a short pane. Raise the cap to the floor
  (`cap.max(floor)`) rather than trusting one to sit above the other.
- Every layout must survive four things, not one: 1024x700, 1280x800,
  `[ui] text_scale = 1.25`, and both docks dragged wide (the docks cap
  against each other so the middle keeps `App::WORK_MIN`).
- **Nothing expensive on the frame.** A view is redrawn sixty times a
  second: a query, a fuzzy pass over the base or a cloned list in a draw
  path is that cost sixty times over. Memoise against the *question* and
  a revision counter the writer moves (`set_drugs`, `table_rev`), never
  against a timestamp; a list row holds an index, not a copy of the row.
  `Session::year_now` reads the year off the date already in hand rather
  than asking the base.
- **A panic is worse than a wrong pixel.** `partial_cmp().unwrap()` on
  computed geometry, `unwrap()` on state that « must » be open,
  `f32::clamp` with a computed min: all of them take the whole
  application down at the counter. Prefer `total_cmp`, an `if let`, and
  a cap raised to its floor.
- The database is shared between PCs: every write to a shared row
  (states, RDV dates, patient identity, deletions) is compare-and-set
  against the values the UI displayed (`WHERE … AND <old values>`,
  returning `bool`; `false` → reload + French notice). UI caches must
  be reloadable, and the team-notes file merges (`merge_team_notes`) —
  never blind last-writer-wins on shared data.

## Env hooks (demo / e2e / screenshots)

- `BPM_CADDY_DB=<path>` — database path override
- `BPM_CADDY_PASSWORD=<pw>` — unlock silently at startup
- `BPM_CADDY_NO_KEYRING=1` — skip the OS credential manager
- `BPM_CADDY_START_VIEW=dashboard|patient|drugs|drug_card|agenda|agenda_day|
  agenda_month|protocols|protocol_open|template|options|about|tables|
  tables_search|calc|carnet|vaccins|bio|watch|revue|conciliation|
  vaccine_map|ordonnance|base|codex|
  codex_open|dispositifs|dispositif_open|locations|keys|vitale|
  act_picker|goto|goto_jump|mono_search|mono_patient`
  — land on a specific view (screenshots, e2e). `about` is the Options
  dialog on its « À propos » page.
- `BPM_CADDY_WINDOW=1280x1100` — open the window at that size
- `BPM_CADDY_DRUG_EDIT=1` — with `START_VIEW=drug_card`, land on the
  editable form rather than the monograph
- `BPM_CADDY_DRUG=<nom>` — with `START_VIEW=drug_card`, open that card
  rather than Eliquis (checking an insulin's action profile, say)
- `BPM_CADDY_VITALE_DUMP=<path>` — replay a captured card instead of
  talking to a reader, so `START_VIEW=vitale` exercises the whole path
  (parsing, matching, the picker) with no hardware and nobody's identity

The workspace's shape — window size, dock widths, whether each dock is
open, the right pane's content, the view on screen — is remembered in
`layout.toml` beside `config.toml`, stamped with the version that wrote
it. The `[ui] show_*_on_start` options decide the *first* launch and
nothing after it, so a test that expects a particular starting shape
must use a throwaway `XDG_CONFIG_HOME` (both scripts already do).

The display settings that break layouts are not env hooks — they are
config: write `[ui] text_scale = 1.25` (or `density = "compact"`) into a
throwaway `XDG_CONFIG_HOME`, and `nav_width` / `docs_width` into
`layout.toml` beside it, to reproduce a wide-dock or large-text screen.
- `BPM_CADDY_SEED_DB=<path> cargo test seed_demo` — create a demo database
- `BPM_CADDY_TEST_PDF_OUT=<dir> cargo test pdf` — write the sample PDF

Xvfb parks the pointer at the centre of the screen, which on a screen
the size of the window is *inside* it: every capture came back with the
tooltip of whatever sat underneath. `screenshots.sh` opens a virtual
screen three times the window's width, keeps the window at its left
edge, and crops back — do the same in any new capture script.

Headless runs: `./scripts/screenshots.sh` regenerates the README
screenshots from a fresh demo seed, and `./scripts/smoke.sh` opens every
view in two shapes and fails on any panic — that is how the Ctrl+N crash
(nine digit keys for ten acts) was found. The second shape is the point:
`f32::clamp` panics when a computed floor crosses a computed cap, and
floors only cross caps on a short pane at large text. A deliberately
inverted clamp in the conciliation pane passes at 1400x900 and brings
the application down at 1024x700 with `text_scale = 1,25` — one pass
would have shipped it. Both shoot against a throwaway
`XDG_CONFIG_HOME`, never the operator's own config. For manual runs:
`xvfb-run` + ImageMagick `import`, and **`unset WAYLAND_DISPLAY` inside
the xvfb shell** or the window opens on the real desktop instead.

## The official bulletins d'adhésion

`assets/bulletins/*.pdf` are the Assurance Maladie's own forms, byte for
byte as ameli.fr serves them — never regenerate, recompress or redraw
them. `src/bulletin.rs` writes their AcroForm fields and nothing else.
The five forms disagree about their field names (and `Adresse 1` means
the patient on three of them and the pharmacy on the other two), so the
mapping is an explicit table read off the rendered pages; the tests in
that module are what keep it honest. Consent boxes, the date and the
signatures are never pre-filled.

## Clinical content

`docs/CONTENU.md` is the map: where each kind of content lives, what
seeds it, what test holds it, and how to add to it. Read it before
adding a card, a posology, a table, a preparation, a protocol, an
analyte or a rule.

`src/ordonnance.rs` and `src/tables.rs` must agree: the antibiotics the
ordonnance offers are the ones the « Angine » and « Cystite » reference
tables list, and `every_molecule_appears_in_its_reference_table` fails
if they drift. Change a protocol in one place and change it in the
other. Nothing there is auto-selected and every posology is editable —
the app proposes, the pharmacist decides. It says so only if the
officine asks it to: every printed or displayed mention lives in
`[disclaimers]` (config.toml, Options › Mentions) and is empty by
default. Never hardcode a new caveat — add a key there.

A new prose field on a drug card is not searchable until it is in
`MONO_FIELDS` (`src/app.rs`) with a label key: « Dans le texte… » reads
that table and nothing else, so a field left out of it is a field nobody
will ever find by its words.

The codex works the same way: `src/db.rs` ships `STARTER_PREPARATIONS`
into the `preparations` table once, and everything after that is the
team's — a formula rewritten in the app is never re-seeded over. Adding
a preparation is adding a fiche, never a second table in the code.

The ordonnance's adjuvants are **not** a list in the code: they are the
drug cards tagged `[ordonnance] adjuvant_tag` (default `probiotique`),
with that card's own posology lines as its schemas. Adding a product is
adding a fiche. Resist any pull to hard-code a second catalogue.

## Releases

Push a `v*` tag → `.github/workflows/release.yml` builds app + launcher
for Linux/Windows/macOS and attaches them to a GitHub Release. Asset
names (`bpm-caddy-linux-x86_64`, `-windows-x86_64.exe`, `-macos-arm64`)
must stay in sync with `APP_ASSET` in `launcher/src/main.rs`. Bump all
three crate versions and move the `Unreleased` changelog section before
tagging.
