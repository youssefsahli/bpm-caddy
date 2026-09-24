#!/usr/bin/env bash
# Ouvrir chaque vue **et s'en servir au hasard**, et échouer sur une
# panique.
#
# `smoke.sh` ouvre une vue et la regarde cinq secondes ; il n'y clique
# jamais. La chute de l'Explorateur (0.316.0) ne se montrait qu'en
# choisissant un organe puis en faisant défiler : ce script-ci lance
# chaque vue avec `BPM_CADDY_FUZZ=<graine>` — des clics, la molette, des
# touches de navigation, quelques caractères, à chaque image — et lit ce
# que l'application écrit.
#
#   ./scripts/fuzz.sh                 # toutes les vues, 20 s chacune
#   ./scripts/fuzz.sh explorer 60     # une vue, une minute
#   FUZZ_SEED=42 ./scripts/fuzz.sh    # une graine choisie, pour rejouer
#   FUZZ_SHAPE=1024x700 FUZZ_SCALE=1.6 ./scripts/fuzz.sh
#
# Les vues sont celles de `smoke.sh`, lues dans son texte : une seule
# liste à tenir. Contre une base de démonstration jetable, jamais celle
# de l'officine.
set -uo pipefail
. "$(dirname "$0")/demo-config.sh"
only=${1:-}
secs=${2:-20}
size=${FUZZ_SHAPE:-1024x700}
scale=${FUZZ_SCALE:-1.25}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export BPM_CADDY_DB="$tmp/demo.db"
export BPM_CADDY_PASSWORD=demo
export BPM_CADDY_NO_KEYRING=1
mkdir -p "$tmp/seed/bpm-caddy"
XDG_CONFIG_HOME="$tmp/seed" \
    BPM_CADDY_SEED_DB="$BPM_CADDY_DB" cargo test seed_demo >/dev/null || exit 1
cargo build || exit 1
bin="$tmp/bpm-caddy"
cp ./target/debug/bpm-caddy "$bin" || exit 1
cp "$BPM_CADDY_DB" "$tmp/pristine.db"

# **Rien ne sort de la boîte.** Le hasard clique aussi « Aperçu PDF »,
# « Ouvrir le dossier » ou la page des versions : les lanceurs du bureau
# sont remplacés par des commandes muettes, le temps de la passe.
mkdir -p "$tmp/path"
for opener in xdg-open gio gnome-open kde-open wslview scanimage; do
    printf '#!/bin/sh\nexit 0\n' > "$tmp/path/$opener"
    chmod +x "$tmp/path/$opener"
done
export PATH="$tmp/path:$PATH"

mapfile -t views < <(
    sed -n '/^views=(/,/^)/p' "$(dirname "$0")/smoke.sh" | sed '1d;$d' | tr -s ' ' '\n' | sed '/^$/d'
)

cfg="$tmp/config"
mkdir -p "$cfg/bpm-caddy"
printf '[ui]\ntext_scale = %s\n' "$scale" > "$cfg/bpm-caddy/config.toml"
: > "$cfg/bpm-caddy/layout.toml"
demo_home "$cfg"
export XDG_CONFIG_HOME="$cfg" BPM_CADDY_WINDOW="$size" bin secs

failed=0
n=0
for view in "${views[@]}"; do
    if [ -n "$only" ] && [ "$only" != "$view" ]; then
        continue
    fi
    n=$((n + 1))
    seed=${FUZZ_SEED:-$((RANDOM * 32768 + RANDOM + n))}
    # Chaque vue repart de la base de démonstration : le hasard écrit.
    cp "$tmp/pristine.db" "$BPM_CADDY_DB"
    rm -f "$BPM_CADDY_DB"-journal
    out=$(
        view="$view" seed="$seed" xvfb-run -a -s "-screen 0 ${size%x*}x${size#*x}x24" bash -c '
            unset WAYLAND_DISPLAY
            case "$view" in
                verrou|search) ;;
                drug_edit) export BPM_CADDY_START_VIEW=drug_card BPM_CADDY_DRUG_EDIT=1 ;;
                drug_kin)  export BPM_CADDY_START_VIEW=drug_card BPM_CADDY_KIN=class ;;
                *)         export BPM_CADDY_START_VIEW="$view" ;;
            esac
            BPM_CADDY_FUZZ="$seed" timeout "$secs" "$bin" 2>&1
        ' | grep -iE -A2 "panicked|out of bounds|unwrap\(\) on" | head -6
    )
    if [ -n "$out" ]; then
        printf '  FAIL  %-16s graine %s\n%s\n' "$view" "$seed" "$(printf '%s' "$out" | sed 's/^/        /')"
        failed=1
    else
        printf '  ok    %-16s graine %s\n' "$view" "$seed"
    fi
done

if [ "$failed" -ne 0 ]; then
    echo "Au moins une vue est tombée : rejouer avec FUZZ_SEED=<graine> ./scripts/fuzz.sh <vue>."
    exit 1
fi
echo "Aucune chute sur $n vue(s)."
