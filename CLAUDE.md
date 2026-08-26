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
  `src/biology.rs` (the analytes, their usual intervals, and the rules
  that read a value against the patient's treatments — static, pure,
  tested), `src/revue.rs` (what a set of treatments says about itself:
  doublons, associations, cascades — same shape, same discipline)
- `launcher/` — `bpm-caddy-launcher`, auto-updates from GitHub Releases
- `motif/` — X/Motif theme for egui (palette, bevels, custom widgets)

Always build/lint with `--workspace`: plain `cargo build` only builds the
root package. CI enforces `cargo fmt --all --check`,
`cargo clippy --workspace -- -D warnings`, `cargo test --workspace`.

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
  `MIGRATIONS` (`src/db.rs`).
- Keep the Motif look: square corners, `motif::bevel` raised for
  buttons/panels, sunken for inputs/troughs; charts are painted by hand,
  no plotting library — `motif::chart` has bars, hbars, stacked,
  sparkline, meter, pips, heat strip and legend.
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
  must survive 1024x700 with both docks open — `scripts/smoke.sh` and a
  screenshot at that size are the check.
- `allocate_new_ui` only sets a max rect and egui paints through it: use
  `motif::inside` when content must not escape its frame.
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
  agenda_month|protocols|protocol_open|template|options|tables|calc|
  carnet|vaccins|bio|revue|vaccine_map|ordonnance|codex|codex_open|keys|
  act_picker`
  — land on a specific view (screenshots, e2e)
- `BPM_CADDY_WINDOW=1280x1100` — open the window at that size
- `BPM_CADDY_DRUG_EDIT=1` — with `START_VIEW=drug_card`, land on the
  editable form rather than the monograph
- `BPM_CADDY_SEED_DB=<path> cargo test seed_demo` — create a demo database
- `BPM_CADDY_TEST_PDF_OUT=<dir> cargo test pdf` — write the sample PDF

Headless runs: `./scripts/screenshots.sh` regenerates the README
screenshots from a fresh demo seed, and `./scripts/smoke.sh` opens every
view once and fails on any panic — that is how the Ctrl+N crash (nine
digit keys for ten acts) was found. Both shoot against a throwaway
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

`src/ordonnance.rs` and `src/tables.rs` must agree: the antibiotics the
ordonnance offers are the ones the « Angine » and « Cystite » reference
tables list, and `every_molecule_appears_in_its_reference_table` fails
if they drift. Change a protocol in one place and change it in the
other. Nothing there is auto-selected and every posology is editable —
the app proposes, the pharmacist decides. It says so only if the
officine asks it to: every printed or displayed mention lives in
`[disclaimers]` (config.toml, Options › Mentions) and is empty by
default. Never hardcode a new caveat — add a key there.

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
