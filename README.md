# BPM-Caddy

**Clinical pharmacy workflow & analytics — fast, local, encrypted.**

BPM-Caddy is a desktop application that streamlines pharmaceutical consultations (BPMs, AODs, asthma interviews) at the dispensing counter. It is built for speed (instant launch, keyboard-first navigation), privacy (fully local, encrypted database — no cloud), and accountability (financial tracking that demonstrates the ROI of clinical activities).

> **Status: in development.** This repository hosts the specification, roadmap, and source code as the project is built. Binaries will be published on the [Releases](../../releases) page.

## Key features

- **Instant fuzzy search** — the app launches straight into a global search bar; typing `jndp` finds *Jean Dupont*. No result? The search seamlessly becomes a patient-creation form.
- **Keyboard-driven workflow** — `Ctrl+F` search, `Enter` select, `Ctrl+N` new interview. Zero loading screens.
- **Native PDF generation with Typst** — clinical interview templates (BPM/AOD frameworks) are typeset in-process by the embedded [Typst](https://typst.app) engine and handed straight to your PDF viewer or printer. No LaTeX, no HTML-to-PDF wrappers.
- **Encrypted at rest** — the patient database is a SQLCipher file (256-bit AES). The key is derived from a master password or stored in the OS credential manager. No external APIs, no telemetry.
- **Billing pipeline state machine** — every interview moves through `Identified → Scheduled → Performed → Report sent → Billed`, so no billable act is ever lost.
- **Financial dashboard** — monthly billed vs. pending revenue, pipeline funnel, and an hourly ROI rate computed from time spent.
- **Portable** — a single standalone executable for Windows, macOS, and Linux. The database path is configurable, so it can live on a secure pharmacy network drive.

## Technology

| Layer | Choice |
|---|---|
| Language | Rust |
| UI | [egui](https://github.com/emilk/egui) (immediate-mode, sub-50 ms startup) |
| Documents | [Typst](https://typst.app) embedded as a Rust crate |
| Database | SQLite + SQLCipher via `rusqlite` |
| Charts | `egui_plot` |

The full requirements document lives in [`docs/SPECIFICATIONS.txt`](docs/SPECIFICATIONS.txt).

## Configuration

BPM-Caddy reads a `config.toml` from the platform config directory:

```toml
[database]
path = "Z:/LGO_Shared/bpm_caddy.db"
auto_lock_timeout_minutes = 15

[ui]
theme = "dark"
default_view = "search"

[templates]
bpm_template_path = "templates/bpm_layout.typ"
```

## Roadmap

- [ ] Application shell: instant-launch egui window, global fuzzy search
- [ ] Patient records: quick creation, encrypted SQLCipher storage
- [ ] Interview lifecycle state machine
- [ ] Typst template engine integration and PDF spooling
- [ ] Financial dashboard (revenue chart, pipeline funnel, hourly ROI)
- [ ] Configuration file and OS credential-manager key storage
- [ ] Packaged releases for Windows / macOS / Linux

## License

BPM-Caddy is **proprietary software with free public releases**: you may use the official binaries free of charge (including professionally) and read the source, but redistribution, modification, and reuse of the code are not permitted. See [LICENSE](LICENSE) for the exact terms.

BPM-Caddy is not a medical device. Users are responsible for compliance with applicable health-data regulations (e.g., GDPR) in their jurisdiction.
