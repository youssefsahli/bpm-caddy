#!/usr/bin/env bash
# Regenerate the README screenshots from a freshly seeded demo database.
# Requires xvfb-run and ImageMagick (import). Run from the repo root:
#   ./scripts/screenshots.sh
set -euo pipefail

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export BPM_CADDY_DB="$tmp/demo.db"
export BPM_CADDY_PASSWORD=demo
export BPM_CADDY_NO_KEYRING=1

BPM_CADDY_SEED_DB="$BPM_CADDY_DB" cargo test seed_demo >/dev/null
cargo build

shot() { # $1 = view (search|dashboard|patient), $2 = output file
    local view=$1 out=$2
    # WAYLAND_DISPLAY must be unset inside the xvfb shell or the window
    # opens on the real desktop instead (see CLAUDE.md).
    view="$view" out="$out" xvfb-run -a -s "-screen 0 1280x800x24" bash -c '
        unset WAYLAND_DISPLAY
        [ "$view" = search ] || export BPM_CADDY_START_VIEW="$view"
        ./target/debug/bpm-caddy &
        app=$!
        sleep 4
        import -window root -crop 1024x700+0+0 +repage "$out"
        kill "$app"
    '
    echo "  $out"
}

shot search docs/screenshot.png
shot dashboard docs/screenshot_dashboard.png
shot patient docs/screenshot_patient.png
shot drugs docs/screenshot_drugs.png
shot agenda docs/screenshot_agenda.png
echo "Screenshots refreshed."
