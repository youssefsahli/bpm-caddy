#!/usr/bin/env bash
# Une seule vue, dans une forme choisie, pour le coup d'œil qu'on répète
# vingt fois pendant qu'on corrige une bande.
#
#   ./scripts/shot.sh <vue> [fichier.png] [taille] [échelle] [clé=valeur…]
#
# Les `clé=valeur` en trop sont écrits dans `layout.toml` — c'est là que
# vit la forme du plan de travail (largeur des volets, bandeau replié).
# Comme `eyeball.sh`, contre un `XDG_CONFIG_HOME` jetable et jamais
# celui de l'opérateur.
set -euo pipefail
# La configuration de démonstration est écrite une fois, pour les deux
# scripts de capture.
. "$(dirname "$0")/demo-config.sh"

view=${1:?usage: shot.sh <vue> [out.png] [taille] [échelle] [clé=valeur…]}
out=${2:-/tmp/bpm-caddy-shot.png}
SIZE=${3:-1024x700}
SCALE=${4:-1.25}
shift 4 2>/dev/null || shift $#

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export BPM_CADDY_DB="$tmp/demo.db"
export BPM_CADDY_PASSWORD=demo
export BPM_CADDY_NO_KEYRING=1

mkdir -p "$tmp/config/bpm-caddy"
: > "$tmp/config/bpm-caddy/layout.toml"
theme=motif
for kv in "$@"; do
    # `theme=` est la seule clé qui vit dans config.toml et non dans la
    # forme du plan de travail — et c'est celle qu'on veut faire varier
    # vingt fois de suite quand on regarde une peau.
    if [ "${kv%%=*}" = theme ]; then
        theme=${kv#*=}
    else
        printf '%s = %s\n' "${kv%%=*}" "${kv#*=}" >> "$tmp/config/bpm-caddy/layout.toml"
    fi
done
demo_config "$tmp/config/bpm-caddy/config.toml" "$SCALE" "$theme"
demo_home "$tmp/config"
export BPM_CADDY_WINDOW="$SIZE"

BPM_CADDY_SEED_DB="$BPM_CADDY_DB" cargo test seed_demo >/dev/null 2>&1
cargo build 2>/dev/null

w=${SIZE%x*} h=${SIZE#*x}
# La carte Vitale rejouée, pour que `shot.sh vitale` montre l'écran du
# lecteur et non son message d'absence.
card="$tmp/vitale.bin"
demo_vitale_card "$card"
# Trois fois la largeur, la fenêtre à gauche, et on recadre : Xvfb gare
# le pointeur au centre de l'écran, donc *dans* la fenêtre si l'écran
# fait sa taille, et chaque capture revenait avec une bulle d'aide.
view="$view" out="$out" w="$w" h="$h" card="$card" \
    xvfb-run -a -s "-screen 0 $((w * 3))x${h}x24" bash -c '
        unset WAYLAND_DISPLAY
        . "'"$(cd "$(dirname "$0")" && pwd)"'/demo-config.sh"
        # Les quatre vues qui demandent autre chose qu une cle : ecrites
        # une fois, dans `demo-config.sh`, et lues par les deux scripts.
        demo_view_env "$view" "$card"
        ./target/debug/bpm-caddy &
        pid=$!
        sleep 6
        import -window root -crop "${w}x${h}+0+0" +repage "$out" 2>/dev/null
        kill $pid 2>/dev/null || true
        wait $pid 2>/dev/null || true
    '
echo "$out"
