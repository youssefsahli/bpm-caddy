# Changelog

All notable changes to BPM-Caddy will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-08-22

### Added
- Time tracking per interview (inline "min" field) feeding the hourly ROI
  KPI ("Taux horaire") on the dashboard
- Master password can be remembered in the OS credential manager (Windows
  Credential Manager, macOS Keychain, Secret Service on Linux) for silent
  unlock at startup; unchecking the box removes the stored copy
- Planned interview dates ("RDV JJ/MM/AAAA" per interview row)
- "Verrouiller" toolbar button and Ctrl+F back-to-search shortcut
- Screenshots in the README, captured from the running app

### Fixed
- Dashboard KPI row no longer overflows narrow windows; monthly chart
  labels months as MM/YY

## [0.2.0] - 2026-08-22

### Added
- Encrypted patient database: SQLCipher (256-bit AES) with a master-password
  unlock screen; wrong passwords are rejected before any data is touched
- Diacritic-insensitive fuzzy patient search ("jndp" finds "Jean Dupont"),
  keyboard navigation (arrows + Enter), and seamless quick-creation form
  (Nom / Prénom / Date de naissance) when no patient matches
- Interview lifecycle state machine (Identifié → Planifié → Réalisé →
  CR envoyé → Facturé) with one-click advancement from the patient view
- `config.toml` support: database path (shareable network drive — the team
  documentation follows the database), auto-lock timeout, per-kind billing
  fees, UI defaults
- Auto-lock: the app returns to the password screen after the configured
  inactivity timeout
- Financial dashboard (F2): billed vs pending revenue KPIs, pipeline
  funnel, and a monthly billed/pending bar chart, all Motif-styled
- Embedded Typst engine: one-click "Fiche PDF" from the patient view
  compiles a single-page A4 interview sheet (patient header + rounded
  boxes for handwritten notes) in memory and opens it in the OS PDF
  viewer; the template is overridable via `[templates]` in `config.toml`

## [0.1.0] - 2026-08-22

### Added
- Project specification (`docs/SPECIFICATIONS.txt`)
- Application skeleton (egui shell)
- Release build pipeline for Windows / macOS / Linux
- `bpm-caddy-launcher`: auto-updating launcher that fetches the latest release
  on startup, with a download progress bar and an offline fallback to the
  installed version
- `motif` crate: old-school X/Motif theme for egui (mwm blue-grey palette,
  square corners, raised/sunken bevels, Motif-style buttons and progress bars),
  applied to both the app and the launcher
- Docked team documentation pane in the app (French, `F1` to toggle,
  debounced auto-save) for shared notes at the counter
