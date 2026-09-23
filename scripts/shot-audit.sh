#!/usr/bin/env bash
# La fenêtre `bpm-audit` sur la base de démonstration, capturée — la même
# configuration jetable que `shot.sh`, et jamais celle de l'opérateur.
#
#   ./scripts/shot-audit.sh [fichier.png] [taille] [échelle]
set -euo pipefail
. "$(dirname "$0")/demo-config.sh"

out=${1:-/tmp/bpm-audit-shot.png}
SIZE=${2:-1100x760}
SCALE=${3:-1.25}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export BPM_CADDY_DB="$tmp/demo.db"
export BPM_CADDY_PASSWORD=demo
export BPM_CADDY_NO_KEYRING=1
mkdir -p "$tmp/config/bpm-caddy"
: > "$tmp/config/bpm-caddy/layout.toml"
demo_config "$tmp/config/bpm-caddy/config.toml" "$SCALE" motif feuille
demo_home "$tmp/config"

BPM_CADDY_SEED_DB="$BPM_CADDY_DB" cargo test seed_demo >/dev/null 2>&1
cargo build --bin bpm-audit 2>/dev/null

w=${SIZE%x*} h=${SIZE#*x}
out="$out" w="$w" h="$h" \
    xvfb-run -a -s "-screen 0 $((w * 3))x${h}x24" bash -c '
        unset WAYLAND_DISPLAY
        ./target/debug/bpm-audit &
        pid=$!
        sleep 5
        import -window root -crop "${w}x${h}+0+0" +repage "$out" 2>/dev/null
        kill $pid 2>/dev/null || true
        wait $pid 2>/dev/null || true
    '
echo "$out"
