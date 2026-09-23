# BPM-Caddy — notes for Claude Code

Clinical pharmacy desktop app (Rust + egui), UI in French, proprietary
license with free public releases. Spec: `docs/SPECIFICATIONS.txt`.

This file is the **working guide**. The reasoning behind every rule — why
each module is shaped the way it is, what each guard test caught, the
full layout doctrine and the env-hook list — is in
**`docs/ARCHITECTURE.md`**. Read the relevant section there before
changing a module, a clinical table, or a view's layout. Clinical
content (cards, posologies, tables, preparations, protocols, analytes,
rules) is mapped in **`docs/CONTENU.md`** — read it before adding any.

`docs/ARCHITECTURE.md` is **read by tests**: `strings.rs` looks up the
`BPM_CADDY_START_VIEW=` list and several counted sentences *verbatim*
(`the_documentation_counts_what_the_code_holds`,
`every_documented_view_is_swept_by_the_smoke_pass`). Rewording a counted
sentence fails the test on purpose; update the sentence and the test
together. A count stated as fact anywhere in the docs must be held by a
line in that test.

## Workspace

- root `bpm-caddy` — a library (`src/lib.rs`) and two binaries: the app
  (`src/main.rs`) and `bpm-audit` (`src/bin/bpm-audit.rs`, the officine's
  audit window, drawn by `src/audit_window.rs`). `src/app.rs` is the UI/state (huge egui
  layout), `src/db.rs` the SQLCipher storage (and the starter content),
  `src/pdf.rs` Typst printing, `src/config.rs` config.toml. Everything
  else in `src/` is a **pure, tested logic module** with no egui and no
  clock (the day/age/DFG is passed in): clinical tables (`renal`,
  `hepatic`, `gravidity`, `elderly`, `crush`, `cyp`, `biology`,
  `surveillance`, `revue`, `conciliation`, `dosing`, `intake`,
  `renewal`, `facets`, `classes`, `vaccines`, `insulin`, `codex`,
  `ordonnance`, `entretien`, `selfcheck`), officine tools
  (`ordonnancier` + `vigilance` = stupéfiants register, `caisse`,
  `planning`, `agenda`, `location`, `prescribers`, `annuaire`,
  `timeline`, `graph`, `scans`, `codebar`, `vitale`/`winscard`,
  `bulletin`, `content`, `script` (Rhai console), `telemetry`, `audit`,
  `release`, `maintenance`, `date`, `fuzzy`, `strings`).
- `launcher/` — `bpm-caddy-launcher`, auto-updates from GitHub Releases;
  does not depend on the app crate.
- `motif/` — X/Motif theme for egui (palette, bevels, widgets, charts).
- `sync/` — `bpm-sync`, the P2P encrypted journal (feature `sync`, on by
  default since 0.273.0). Only one use is wired: the officines' network
  (`src/network.rs`), which carries `Stream::Reseau` — shortages and
  substitutions, never a patient — under a network-only trousseau. Map:
  `docs/SYNC.md`.

## Build and gates

Always use `--workspace` (plain `cargo build` builds only the root).
CI enforces, and a change is not done until all pass:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/coverage.sh      # two floors that only move up
./scripts/smoke.sh         # every view, four shapes, fails on panic (~70 min)
```

`target/` grows past 70 GB and fills the disk; a full disk shows up as a
linker bus error or a silently killed build. Run `df -h /` before long
runs; `rm -rf target/debug/incremental target/llvm-cov-target` is the
cheap reclaim, `cargo clean` the full one. Do not rebuild while
`smoke.sh` runs — it builds once and runs the frozen binary.

## Conventions (the short form — reasons in ARCHITECTURE.md)

**Strings and dates**
- UI strings are French, code/comments English. Every UI string lives
  in `assets/strings.fr.toml` via `strings::tr/trf/trn` — never
  hardcode one. Terse professional register.
- Dates stored ISO, displayed `JJ/MM/AAAA`; input through
  `db::parse_french_date` + `db::YearHint` (accepts `230826`, `2308`).
- Only characters the bundled egui faces have a glyph for (no arrows in
  proportional text; no U+202F). Glyph tests guard this.

**Storage**
- Schema change = `SCHEMA` **and** an idempotent `ALTER TABLE` in
  `MIGRATIONS`; `a_base_from_an_older_version_still_answers_every_query`
  catches the forgotten one.
- The base is shared between posts: every write to a shared row is
  compare-and-set against what the UI displayed; `false` → reload +
  notice through `Session::stale`/`stale_note` (never written by hand).
  `resync` reloads readings, never a typing buffer.
- Officine-wide settings live in the base (`settings`); per-post ones in
  `config.toml`.
- Three encrypted files travel together: base, `_scans.db`, `_stups.db`.
  `change_password`, copy, backup and export handle all three.
- The stupéfiants register is **INSERT-only** (R. 5132-36): no UPDATE,
  no DELETE; a correction is a `Kind::Annulation` line. A test reads
  `db.rs` and refuses the verbs. Register lines carry a file number,
  never a patient name.
- Scanned pieces are identified by magic bytes, never by extension.
- Each test gets its own uniquely-named temp directory.

**Clinical tables**
- Keyed on the molecule; the route is asked once, through
  `classes::is_local_form`. Per-row `never` vetoes for local forms.
- A `needs` fragment is a substring: before trusting one, list every
  shipped card it catches (« ipp » is inside « grippe »). Name
  molecules, not class abbreviations.
- Backing tests judge every card against the **first** row that claims
  it. Tables that run whole (`biology`, `revue`) must not hold two rules
  that give one reading.
- Silence is not permission: an unknown answers « à vérifier ». No
  invented figures, no posology the source does not write, the
  prescriber decides. Caveats live in `[disclaimers]`, never hardcoded.
- Every printed phrase is rewritable (`src/content.rs`): a module
  exposes `phrases()` + `resolve()`, addresses derive from rule
  identity, and the paired test goes both ways.
- `src/ordonnance.rs` and `src/tables.rs` must agree; adjuvants and
  preparations are cards/rows in the base, never a second list in code.
- A new prose card field goes in `MONO_FIELDS` or it is unsearchable.

**Printing**
- Every printable is one entry in `pdf::DOCS` with a Typst default and
  its markers; four tests hold markers, compilation and `template_path`.
- `assets/bulletins/*.pdf` are the official forms byte for byte; only
  their AcroForm fields are written.

**Look and layout** (egui, Motif chrome)
- Square corners; raised bevel = pressed, sunken = filled in.
  Use `motif::field`/`area`/`select`/`menu`/`scale_range`/`separator` —
  never flat egui `TextEdit` frames, `ComboBox`, `Slider` or
  `ui.separator()`. Charts are hand-painted in `motif::chart`.
- Colour from the theme (`motif::bg()`, `accent()`…, `data_ramp`), never
  a literal; rules about colour are distances (two palettes are dark).
  Hints through `motif::hint`. A verdict colour also carries a `Pict`.
- Font size via `motif::pt`, field width via `chars_wide`/`field_width`,
  layout switches in characters, row height via `button_height` /
  `row_height` / `label_line` — never pixel literals. Source-reading
  tests refuse each of these.
- Layout is carved (`split_rows`/`split_columns`, `panel`, `inside`),
  measured against `motif::visible_rect`. Measure with the width the
  drawing uses (`App::scrolled_width` under a scroll bar); write a
  cell's text once for both measuring and drawing; caps are a share of
  the pane and land on whole rows; `cap.max(floor)`, never a clamp that
  can invert. Say the total when a band is cut; turn the floating bar
  off where the hidden tail is a gesture.
- Every layout must survive 1024x700, 1280x800, `text_scale` 1.25 and
  1.6, and both docks dragged wide. Second `ScrollArea` in a view needs
  `.id_salt`.
- Nothing expensive per frame: memoise against the question and a
  revision counter. No panics on geometry (`total_cmp`, no `unwrap`).
- **A change to a screen has companions**: its measurement, its test's
  measurement, what it copies, what it prints, what `assets/aide.md`
  says. Helpers that measure can be tested headless (`egui::Context` in a
  plain `#[test]`) — draw, then compare promise to occupancy both ways.

## Looking at the UI

Headless under Xvfb with `unset WAYLAND_DISPLAY`, a throwaway
`XDG_CONFIG_HOME`, and env hooks (`BPM_CADDY_DB`, `BPM_CADDY_PASSWORD`,
`BPM_CADDY_NO_KEYRING=1`, `BPM_CADDY_START_VIEW=<key>`,
`BPM_CADDY_WINDOW=WxH`; full list and per-view notes in ARCHITECTURE.md).

- `./scripts/shot.sh <vue> [fichier] [taille] [échelle] [clé=valeur…]` —
  one view (`theme=`, `mono=` go to config.toml, the rest to layout.toml).
- `./scripts/eyeball.sh [dir] 1024x700 1.6` — every view; **look at the
  pictures**. `vierge=1` shows the first-launch empty base.
- `./scripts/screenshots.sh` — README screenshots.
- A new view key goes in the ARCHITECTURE.md list, `smoke.sh` and
  `eyeball.sh` (a test holds the three together).

## Releases

Push a `v*` tag → `.github/workflows/release.yml` builds app + launcher
for Linux/Windows/macOS. Asset names must match `APP_ASSET` in
`launcher/src/main.rs`. Bump all three crate versions and move the
`Unreleased` changelog section before tagging.
