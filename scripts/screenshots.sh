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
    view="$view" out="$out" extra="$*" \
    xvfb-run -a -s "-screen 0 ${SIZE%x*}x${SIZE#*x}x24" bash -c '
        unset WAYLAND_DISPLAY
        [ "$view" = search ] || export BPM_CADDY_START_VIEW="$view"
        for kv in $extra; do export "$kv"; done
        ./target/debug/bpm-caddy &
        app=$!
        sleep 4
        import -window root +repage "$out"
        kill "$app"
    '
    echo "  $out"
}

shot search docs/screenshot.png
shot dashboard docs/screenshot_dashboard.png
shot patient docs/screenshot_patient.png
shot drug_card docs/screenshot_drugs.png
shot agenda docs/screenshot_agenda.png
shot tables docs/screenshot_tables.png
echo "Screenshots refreshed."
