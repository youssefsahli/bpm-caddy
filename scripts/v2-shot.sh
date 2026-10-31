#!/usr/bin/env bash
# Une vue de la fenêtre v2, sur la base de démonstration.
#
#   ./scripts/v2-shot.sh <vue> [fichier.png] [taille] [échelle] [thème]
#
# Vues : accueil patients patient patient_new medicaments fiche verifier
# verifier_patient agenda stups options verrou. Sans Xvfb sur la machine :
# PATH="$PWD/scripts/headless:$PATH" ./scripts/v2-shot.sh …
set -euo pipefail
. "$(dirname "$0")/demo-config.sh"

view=${1:?usage: v2-shot.sh <vue> [out.png] [taille] [échelle] [thème]}
out=${2:-/tmp/bpm-caddy-v2.png}
SIZE=${3:-1024x700}
SCALE=${4:-1.0}
THEME=${5:-motif}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export BPM_CADDY_DB="$tmp/demo.db"
export BPM_CADDY_PASSWORD=demo
export BPM_CADDY_NO_KEYRING=1
mkdir -p "$tmp/config/bpm-caddy"
demo_config "$tmp/config/bpm-caddy/config.toml" "$SCALE" "$THEME" feuille
sed -i 's/^\[ui\]$/[ui]\noperator = "CL"/' "$tmp/config/bpm-caddy/config.toml"
demo_home "$tmp/config"
export BPM_CADDY_WINDOW="$SIZE"
export BPM_CADDY_START_VIEW="$view"

BPM_CADDY_SEED_DB="$BPM_CADDY_DB" cargo test -p bpm-caddy --lib seed_demo >/dev/null 2>&1
cargo build -p bpm-caddy-v2 2>/dev/null

w=${SIZE%x*} h=${SIZE#*x}
out="$out" w="$w" h="$h" \
    xvfb-run -a -s "-screen 0 $((w * 3))x${h}x24" bash -c '
        unset WAYLAND_DISPLAY
        ./target/debug/bpm-caddy-v2 2>"${SHOT_ERR:-/dev/null}" &
        pid=$!
        sleep ${SHOT_WAIT:-5}
        import -window root "$out" 2>/dev/null
        kill $pid 2>/dev/null || true
        wait $pid 2>/dev/null || true
    '
# Recadré ici, hors du serveur X : la fenêtre est à gauche d'un écran
# trois fois plus large (le pointeur garé au centre n'y pose aucune bulle).
magick "$out" -crop "${w}x${h}+0+0" +repage "$out" 2>/dev/null \
    || convert "$out" -crop "${w}x${h}+0+0" +repage "$out"
echo "$out"
