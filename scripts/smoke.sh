#!/usr/bin/env bash
# Open every view once against a demo database and fail on any panic.
#
# The views are reached through BPM_CADDY_START_VIEW, so a code path
# that only runs when a dialog is open — the quick picker, the options,
# the templates — is exercised too. That is how the Ctrl+N crash was
# found: nine digit keys for ten acts, panicking on the first frame the
# picker was drawn.
#
# Requires xvfb-run. Run from the repo root:
#   ./scripts/smoke.sh
set -uo pipefail

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export BPM_CADDY_DB="$tmp/demo.db"
export BPM_CADDY_PASSWORD=demo
export BPM_CADDY_NO_KEYRING=1
export BPM_CADDY_WINDOW=1400x900
# Never touch the operator's own configuration.
mkdir -p "$tmp/config/bpm-caddy"
export XDG_CONFIG_HOME="$tmp/config"

BPM_CADDY_SEED_DB="$BPM_CADDY_DB" cargo test seed_demo >/dev/null || exit 1
cargo build || exit 1

views=(
    search dashboard patient drugs drug_card drug_edit
    agenda agenda_day agenda_month tables tables_search calc carnet
    vaccins bio revue vaccine_map ordonnance
    protocols protocol_open codex codex_open template options keys act_picker
    goto goto_jump mono_search mono_patient
)

failed=0
for view in "${views[@]}"; do
    out=$(
        view="$view" xvfb-run -a -s "-screen 0 1400x900x24" bash -c '
            unset WAYLAND_DISPLAY
            case "$view" in
                drug_edit) export BPM_CADDY_START_VIEW=drug_card BPM_CADDY_DRUG_EDIT=1 ;;
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

if [ "$failed" -ne 0 ]; then
    echo "Smoke test failed."
    exit 1
fi
echo "Every view opened without panicking."
