#!/usr/bin/env bash
# Capture every view in the shape that breaks layouts — 1024x700 with
# `[ui] text_scale = 1.25` — into a directory, for the eye check that
# `smoke.sh` cannot do.
#
# `smoke.sh` proves nothing panicked. It says nothing about a button
# drawn half off a panel, a heading that wrapped onto the row under it,
# or eight doors reflowing into four lines. Those are found by looking,
# and looking is only cheap if the pictures are one command away.
#
#   ./scripts/eyeball.sh [outdir] [size] [scale]
#
# Requires xvfb-run and ImageMagick. Run from the repo root.
set -euo pipefail

out=${1:-/tmp/bpm-caddy-eyeball}
SIZE=${2:-1024x700}
SCALE=${3:-1.25}
mkdir -p "$out"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export BPM_CADDY_DB="$tmp/demo.db"
export BPM_CADDY_PASSWORD=demo
export BPM_CADDY_NO_KEYRING=1

# A throwaway configuration, never the operator's own.
mkdir -p "$tmp/config/bpm-caddy"
cat > "$tmp/config/bpm-caddy/config.toml" <<EOF
[ui]
discreet_finances = false
text_scale = $SCALE
EOF
export XDG_CONFIG_HOME="$tmp/config"
export BPM_CADDY_WINDOW="$SIZE"

BPM_CADDY_SEED_DB="$BPM_CADDY_DB" cargo test seed_demo >/dev/null
cargo build

card="$tmp/vitale.bin"
printf 'ceci n%s est pas une carte' "'" > "$card"

views=(
    search dashboard patient drugs drug_card drug_edit drug_kin
    agenda agenda_day agenda_month tables tables_search calc carnet
    vaccins bio watch revue locations conciliation vaccine_map ordonnance
    protocols protocol_open codex codex_open dispositifs dispositif_open
    graph stup stup_catalogue ordonnancier vigilance scans patient_scans registres
    explorer explorer_organ classes classes_outside export
    template options about base keys act_picker
    goto goto_jump mono_search mono_patient
)

w=${SIZE%x*} h=${SIZE#*x}
for view in "${views[@]}"; do
    # The virtual screen is three times the window wide and the window
    # sits at its left edge: Xvfb parks the pointer in the middle, which
    # on a screen the size of the window is *inside* it, and every shot
    # came back with whatever tooltip happened to be under it.
    view="$view" out="$out" w="$w" h="$h" card="$card" \
    xvfb-run -a -s "-screen 0 $((w * 3))x${h}x24" bash -c '
        unset WAYLAND_DISPLAY
        case "$view" in
            drug_edit) export BPM_CADDY_START_VIEW=drug_card BPM_CADDY_DRUG_EDIT=1 ;;
            drug_kin)  export BPM_CADDY_START_VIEW=drug_card BPM_CADDY_KIN=class ;;
            search)    ;;
            *)         export BPM_CADDY_START_VIEW="$view" ;;
        esac
        ./target/debug/bpm-caddy &
        app=$!
        sleep 3
        import -window root +repage -crop "${w}x${h}+0+0" +repage \
            "$out/$view.png" 2>/dev/null
        kill "$app" 2>/dev/null
        wait "$app" 2>/dev/null || true
    '
    echo "  $out/$view.png"
done
echo
echo "Regardez-les : $out"
