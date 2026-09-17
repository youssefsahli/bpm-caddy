#!/usr/bin/env bash
# Le compagnon sur une fenêtre **maximisée**, qui est une forme et non
# une taille.
#
# `smoke.sh` et `eyeball.sh` tournent sous un Xvfb **sans gestionnaire de
# fenêtres** : rien n'y est jamais maximisé, et une demande de taille y
# est donc toujours honorée. Sur un poste réel c'est l'inverse — le
# gestionnaire tient la taille d'une fenêtre maximisée et jette la
# demande — et c'est ainsi que F9 laissait la fenêtre pleine au lieu d'en
# faire une barre, sur le seul genre de poste où l'application est
# toujours maximisée : un comptoir.
#
# Ce script monte donc un vrai compositeur. Il demande `mutter`, ce qui
# est exactement la raison pour laquelle il n'est **pas** dans les deux
# passes de balayage : celles-ci ne doivent rien exiger de plus que Xvfb.
#
#   ./scripts/maximized.sh
#
# Ce qu'il faut y lire, dans l'ordre :
#
#   companion=true  rect=[1024 665] max=Some(true)    la fenêtre pleine
#   companion=true  rect=[1024 665] max=Some(false)   démaximisée
#   companion=true  rect=[560 420]  max=Some(false)   la barre
#
# Deux images séparent la démaximisation de la taille, et ce n'est pas
# une précaution : mesuré ici, le compositeur met deux images à rendre la
# fenêtre. Les deux ordres envoyés dans la même image se marchent dessus,
# et la taille est jetée comme avant.
set -uo pipefail
. "$(dirname "$0")/demo-config.sh"

command -v mutter >/dev/null || {
    echo "Il faut un compositeur : ce script demande mutter." >&2
    exit 1
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

export BPM_CADDY_DB="$tmp/demo.db"
export BPM_CADDY_PASSWORD=demo
export BPM_CADDY_NO_KEYRING=1
mkdir -p "$tmp/config/bpm-caddy"
: > "$tmp/config/bpm-caddy/layout.toml"
demo_config "$tmp/config/bpm-caddy/config.toml" 1.0 motif
demo_home "$tmp/config"
BPM_CADDY_SEED_DB="$BPM_CADDY_DB" cargo test seed_demo >/dev/null 2>&1 || exit 1
cargo build || exit 1

export XDG_RUNTIME_DIR="$tmp/run"
mkdir -p "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
mutter --headless --virtual-monitor 1024x700 > "$tmp/mutter.log" 2>&1 &
wm=$!
sleep 6
if ! grep -q "Wayland display name" "$tmp/mutter.log"; then
    echo "Le compositeur n'a pas démarré :" >&2
    head -10 "$tmp/mutter.log" >&2
    kill $wm 2>/dev/null
    exit 1
fi

# **Client Wayland** : ce qu'on lit ici n'est pas une image mais ce que
# l'application voit de l'état de sa fenêtre, et passer par Xwayland
# demanderait un cookie que ce montage n'a pas.
unset DISPLAY
export WAYLAND_DISPLAY=wayland-0
export BPM_CADDY_MAXIMIZED=1
export BPM_CADDY_START_VIEW=companion
timeout 20 ./target/debug/bpm-caddy > "$tmp/app.log" 2>&1
kill $wm 2>/dev/null

echo "--- ce que l'application a vu de sa fenêtre ---"
grep -E "rect=" "$tmp/app.log" | awk '!seen[$0]++' | head -10
if [ ! -s "$tmp/app.log" ]; then
    echo "(rien : l'application n'a pas démarré sous le compositeur)"
fi
