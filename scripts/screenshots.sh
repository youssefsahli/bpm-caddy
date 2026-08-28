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

# Shoot against a throwaway configuration, never the operator's own:
# discreet mode would otherwise mask every figure on the dashboard, and
# the run would leave the real config.toml rewritten.
mkdir -p "$tmp/config/bpm-caddy"
cat > "$tmp/config/bpm-caddy/config.toml" <<'EOF'
[ui]
discreet_finances = false
EOF
export XDG_CONFIG_HOME="$tmp/config"

# The workspace is three docks and a notebook around the work: shoot it
# at the width a counter screen actually has, not at 1024x700.
SIZE=${SIZE:-1600x1000}
export BPM_CADDY_WINDOW="$SIZE"

BPM_CADDY_SEED_DB="$BPM_CADDY_DB" cargo test seed_demo >/dev/null
cargo build

shot() { # $1 = view, $2 = output file, $3.. = extra VAR=value env
    local view=$1 out=$2
    shift 2
    # WAYLAND_DISPLAY must be unset inside the xvfb shell or the window
    # opens on the real desktop instead (see CLAUDE.md).
    # The virtual screen is three times the window wide, and the window
    # sits at its left edge. Xvfb parks the pointer in the middle of the
    # screen, which — on a screen the size of the window — is inside it:
    # every shot came back with whatever tooltip happened to be under
    # it. Off to the right, the pointer hovers nothing, and the image is
    # cropped back to the window.
    local w=${SIZE%x*} h=${SIZE#*x}
    view="$view" out="$out" extra="$*" w="$w" h="$h" \
    xvfb-run -a -s "-screen 0 $((w * 3))x${h}x24" bash -c '
        unset WAYLAND_DISPLAY
        [ "$view" = search ] || export BPM_CADDY_START_VIEW="$view"
        for kv in $extra; do export "$kv"; done
        ./target/debug/bpm-caddy &
        app=$!
        sleep 4
        import -window root +repage -crop "${w}x${h}+0+0" +repage "$out"
        kill "$app"
    '
    echo "  $out"
}

shot search docs/screenshot.png
shot goto docs/screenshot_goto.png
shot mono_search docs/screenshot_mono.png
shot dashboard docs/screenshot_dashboard.png
shot patient docs/screenshot_patient.png
shot drug_card docs/screenshot_drugs.png
shot agenda docs/screenshot_agenda.png
shot vaccins docs/screenshot_vaccins.png
shot bio docs/screenshot_bio.png
shot codex_open docs/screenshot_codex.png
shot vaccine_map docs/screenshot_map.png
shot tables docs/screenshot_tables.png
# The three carved list views, and the insulin panel under the
# calculators: added when they were, so the README stops showing an
# application older than the one it ships.
shot protocol_open docs/screenshot_protocols.png
shot dispositif_open docs/screenshot_dispositifs.png
shot locations docs/screenshot_locations.png
shot conciliation docs/screenshot_conciliation.png
echo "Screenshots refreshed."
