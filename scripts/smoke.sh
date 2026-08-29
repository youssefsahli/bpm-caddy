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

views=(
    search dashboard patient drugs drug_card drug_edit
    agenda agenda_day agenda_month tables tables_search calc carnet
    vaccins bio watch revue locations conciliation vaccine_map ordonnance
    protocols protocol_open codex codex_open dispositifs dispositif_open
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
