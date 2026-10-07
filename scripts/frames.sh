#!/usr/bin/env bash
# Le coût d'une image, vue par vue, en version optimisée.
#
#   ./scripts/frames.sh [taille] [échelle] [vue…]
#
# Chaque vue tourne quelques secondes (FRAMES_WAIT, 6 par défaut) sous
# un serveur X sans écran, avec `BPM_CADDY_FRAME_STATS=1` : l'application
# redessine sans arrêt et écrit, toutes les 60 images, la moyenne et le
# maximum du temps passé dans `update` (la mise en page egui, sans le
# rendu GPU). Le script garde la dernière ligne de chaque vue et trie les
# vues de la plus lente à la plus rapide. Sans vue nommée, toutes celles
# de `smoke.sh`.
set -euo pipefail
. "$(dirname "$0")/demo-config.sh"
SIZE=${1:-1280x800}
SCALE=${2:-1.0}
shift 2 2>/dev/null || shift $#
if [ $# -gt 0 ]; then
    views=("$@")
else
    read -r -a views <<< "$(sed -n '/^views=(/,/^)/p' "$(dirname "$0")/smoke.sh" | grep -v '^views=(\|^)' | tr '\n' ' ')"
fi
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export BPM_CADDY_DB="$tmp/demo.db" BPM_CADDY_PASSWORD=demo BPM_CADDY_NO_KEYRING=1
mkdir -p "$tmp/config/bpm-caddy"
: > "$tmp/config/bpm-caddy/layout.toml"
demo_config "$tmp/config/bpm-caddy/config.toml" "$SCALE" motif feuille
demo_home "$tmp/config"
BPM_CADDY_SEED_DB="$BPM_CADDY_DB" cargo test seed_demo >/dev/null 2>&1
cargo build --release >/dev/null 2>&1
cp "$BPM_CADDY_DB" "$tmp/pristine.db"
w=${SIZE%x*} h=${SIZE#*x}
card="$tmp/vitale.bin"
demo_vitale_card "$card"
out="$tmp/results"
: > "$out"
for view in "${views[@]}"; do
    cp "$tmp/pristine.db" "$BPM_CADDY_DB"
    log="$tmp/$view.log"
    view="$view" w="$w" h="$h" card="$card" log="$log" BPM_CADDY_WINDOW="$SIZE" \
        xvfb-run -a -s "-screen 0 $((w * 3))x${h}x24" bash -c '
            unset WAYLAND_DISPLAY
            . "'"$(cd "$(dirname "$0")" && pwd)"'/demo-config.sh"
            demo_view_env "$view" "$card"
            BPM_CADDY_FRAME_STATS=1 ./target/release/bpm-caddy 2>"$log" &
            pid=$!
            sleep ${FRAMES_WAIT:-6}
            kill $pid 2>/dev/null || true
            wait $pid 2>/dev/null || true
        ' || true
    line=$(grep frame-stats "$log" | tail -1 || true)
    avg=$(echo "$line" | sed -n 's/.*avg_us=\([0-9]*\).*/\1/p')
    max=$(echo "$line" | sed -n 's/.*max_us=\([0-9]*\).*/\1/p')
    printf '%8s %9s  %s\n' "${avg:--}" "${max:--}" "$view" | tee -a "$out"
done
echo "--- de la plus lente à la plus rapide (moyenne µs, maximum µs)"
sort -rn "$out"
