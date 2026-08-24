# BPM-Caddy — notes for Claude Code

Clinical pharmacy desktop app (Rust + egui), UI in French, proprietary
license with free public releases. Spec: `docs/SPECIFICATIONS.txt`.

## Workspace

- root `bpm-caddy` — the app: `src/app.rs` (UI/state), `src/db.rs`
  (SQLCipher storage), `src/fuzzy.rs` (search), `src/pdf.rs` (Typst),
  `src/config.rs` (config.toml)
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
  no plotting library.
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
- `BPM_CADDY_START_VIEW=dashboard|patient|drugs|drug_card|agenda|template`
  — land on a specific view (screenshots, e2e)
- `BPM_CADDY_SEED_DB=<path> cargo test seed_demo` — create a demo database
- `BPM_CADDY_TEST_PDF_OUT=<dir> cargo test pdf` — write the sample PDF

Headless run (screenshots): `./scripts/screenshots.sh` regenerates the
three README screenshots from a fresh demo seed. For manual runs:
`xvfb-run` + ImageMagick `import`, and **`unset WAYLAND_DISPLAY` inside
the xvfb shell** or the window opens on the real desktop instead.

## Releases

Push a `v*` tag → `.github/workflows/release.yml` builds app + launcher
for Linux/Windows/macOS and attaches them to a GitHub Release. Asset
names (`bpm-caddy-linux-x86_64`, `-windows-x86_64.exe`, `-macos-arm64`)
must stay in sync with `APP_ASSET` in `launcher/src/main.rs`. Bump all
three crate versions and move the `Unreleased` changelog section before
tagging.
