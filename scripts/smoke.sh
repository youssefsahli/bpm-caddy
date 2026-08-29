#!/usr/bin/env bash
# Open every view against a demo database and fail on any panic.
#
# The views are reached through BPM_CADDY_START_VIEW, so a code path
# that only runs when a dialog is open — the quick picker, the options,
# the templates — is exercised too. That is how the Ctrl+N crash was
# found: nine digit keys for ten acts, panicking on the first frame the
# picker was drawn.
#
# Every view is opened in **two shapes**, because the panic this is
# looking for does not happen in the first one. `f32::clamp` takes the
# whole application down when a computed floor crosses a computed cap,
# and floors only cross caps on a short pane at large text — which is
# exactly the shape a counter screen has and a developer's does not.
# CLAUDE.md asks for 1024x700 and text_scale 1,25 as a manual check;
# this is that check, run every time.
#
# Requires xvfb-run. Run from the repo root:
#   ./scripts/smoke.sh
set -uo pipefail

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export BPM_CADDY_DB="$tmp/demo.db"
export BPM_CADDY_PASSWORD=demo
export BPM_CADDY_NO_KEYRING=1

# A captured card, so the reader path runs on every post: no hardware,
# no real card, nobody's identity.
card="$tmp/card.bin"
export card
printf '\x00DUPONT\x00JEAN\x00155087511600125\x0003081955\x00' > "$card"

# Never touch the operator's own configuration: a throwaway one per
# shape, so each pass starts from the same known state instead of
# inheriting the docks and the window the previous pass left in
# layout.toml.
mkdir -p "$tmp/seed/bpm-caddy"
XDG_CONFIG_HOME="$tmp/seed" \
    BPM_CADDY_SEED_DB="$BPM_CADDY_DB" cargo test seed_demo >/dev/null || exit 1
cargo build || exit 1

# Prove the application starts before believing seventy-six silent runs.
#
# The check below is « no panic line came out », and a binary that dies
# before it draws anything produces no panic line either: a missing
# system library, a display that did not come up, a build that did not
# refresh — every one of them reads as a clean pass. `timeout` returns
# 124 when it had to kill something that was still alive, and anything
# else means the process ended on its own.
mkdir -p "$tmp/alive/bpm-caddy"
alive="$tmp/alive.log"
XDG_CONFIG_HOME="$tmp/alive" BPM_CADDY_WINDOW=1400x900 \
    xvfb-run -a -s "-screen 0 1400x900x24" bash -c '
        unset WAYLAND_DISPLAY
        timeout 5 ./target/debug/bpm-caddy' > "$alive" 2>&1
code=$?
if [ "$code" -ne 124 ]; then
    echo "L'application ne reste pas ouverte cinq secondes : un passage" >&2
    echo "sans panique ne prouverait rien." >&2
    echo "  (le programme s'est arrêté de lui-même, code $code)" >&2
    # What it said on the way out. A guard that only reports *that* it
    # failed sends whoever reads it guessing at system packages; this is
    # the line that names the missing one.
    if [ -s "$alive" ]; then
        sed 's/^/  | /' "$alive" >&2
    else
        echo "  | (rien sur la sortie d'erreur)" >&2
    fi
    exit 1
fi

views=(
    search dashboard patient drugs drug_card drug_edit drug_kin
    agenda agenda_day agenda_month tables tables_search calc carnet
    vaccins bio watch revue locations conciliation vaccine_map ordonnance
    protocols protocol_open codex codex_open dispositifs dispositif_open graph stup scans patient_scans registres
    template options about base keys act_picker vitale
    goto goto_jump mono_search mono_patient
)

# name : window : the [ui] block that shape needs.
shapes=(
    "ordinaire|1400x900|"
    "comptoir|1024x700|text_scale = 1.25"
)

failed=0
for shape in "${shapes[@]}"; do
    IFS="|" read -r shape_name size ui <<< "$shape"
    export BPM_CADDY_WINDOW="$size"
    cfg="$tmp/config-$shape_name"
    mkdir -p "$cfg/bpm-caddy"
    printf '[ui]\n%s\n' "$ui" > "$cfg/bpm-caddy/config.toml"
    export XDG_CONFIG_HOME="$cfg"
    printf '\n  --- %s (%s%s) ---\n' "$shape_name" "$size" \
        "${ui:+, $ui}"
    for view in "${views[@]}"; do
        out=$(
            view="$view" size="$size" xvfb-run -a -s "-screen 0 ${size%x*}x${size#*x}x24" bash -c '
                unset WAYLAND_DISPLAY
                case "$view" in
                    drug_edit) export BPM_CADDY_START_VIEW=drug_card BPM_CADDY_DRUG_EDIT=1 ;;
                    # The technical pane with a neighbour list unfolded:
                    # its tallest shape, and the one whose height is
                    # computed rather than fixed.
                    drug_kin)  export BPM_CADDY_START_VIEW=drug_card BPM_CADDY_KIN=class ;;
                    # The card reader is exercised on a captured dump, never
                    # on a real card: the whole path runs, nothing is plugged
                    # in, and no patient of anybody is read.
                    vitale)    export BPM_CADDY_START_VIEW=vitale
                               export BPM_CADDY_VITALE_DUMP="$card" ;;
                    search)    ;;
                    *)         export BPM_CADDY_START_VIEW="$view" ;;
                esac
                timeout 5 ./target/debug/bpm-caddy 2>&1
            ' | grep -iE "panicked|out of bounds|unwrap\(\) on" | head -3
        )
        if [ -n "$out" ]; then
            printf '  FAIL  %-14s %s\n' "$view" "$out"
            failed=1
        else
            printf '  ok    %s\n' "$view"
        fi
    done
done

if [ "$failed" -ne 0 ]; then
    echo "Smoke test failed."
    exit 1
fi
echo "Every view opened without panicking, in both shapes."
