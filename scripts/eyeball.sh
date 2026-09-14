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
#   ./scripts/eyeball.sh [outdir] [size] [scale] [peau] [clé=valeur…]
#
# La peau est le quatrième argument parce que la couleur se regarde comme
# la mise en page : rien ne dit dans un test qu'une teinte s'est perdue
# dans son fond.
#
# Les `clé=valeur` en trop vont dans `layout.toml`, comme pour `shot.sh` :
# c'est là que vit la forme du plan de travail. Sans eux, les volets
# prennent leur largeur par défaut, et **les deux volets tirés larges**
# — l'une des quatre formes que CLAUDE.md exige de toute mise en page —
# n'était produite par aucun script. Elle se balaie maintenant d'une
# commande :
#
#   ./scripts/eyeball.sh /tmp/larges 1280x800 1.0 motif \
#       nav_width=420 docs_width=420
#
# Requires xvfb-run and ImageMagick. Run from the repo root.
set -euo pipefail
# La même configuration de démonstration que `shot.sh` : les deux
# scripts regardent les mêmes vues, ils doivent en montrer le même état.
. "$(dirname "$0")/demo-config.sh"

out=${1:-/tmp/bpm-caddy-eyeball}
SIZE=${2:-1024x700}
SCALE=${3:-1.25}
THEME=${4:-motif}
shift 4 2>/dev/null || shift $#
mkdir -p "$out"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export BPM_CADDY_DB="$tmp/demo.db"
export BPM_CADDY_PASSWORD=demo
export BPM_CADDY_NO_KEYRING=1

# A throwaway configuration, never the operator's own.
mkdir -p "$tmp/config/bpm-caddy"
# La forme du plan de travail : vide par défaut, sinon ce qu'on a demandé.
#
# **Réécrite avant chaque vue.** L'application enregistre la forme du
# plan de travail en quittant : sans remise à zéro, chaque capture
# héritait de la précédente — « aide », qui n'est pas une vue mais un
# onglet du volet droit, s'ouvrait sur ce que la vue d'avant avait
# laissé, et réordonner la liste changeait des images. Une capture doit
# ne dépendre que de sa vue.
layout() {
    : > "$tmp/config/bpm-caddy/layout.toml"
    for kv in "$@"; do
        printf '%s = %s\n' "${kv%%=*}" "${kv#*=}" >> "$tmp/config/bpm-caddy/layout.toml"
    done
}
layout "$@"
demo_config "$tmp/config/bpm-caddy/config.toml" "$SCALE" "$THEME"
demo_home "$tmp/config"
export BPM_CADDY_WINDOW="$SIZE"

BPM_CADDY_SEED_DB="$BPM_CADDY_DB" cargo test seed_demo >/dev/null
cargo build

card="$tmp/vitale.bin"
demo_vitale_card "$card"

views=(
    search dashboard patient patient_edit patient_new drugs drug_card drug_edit drug_kin
    agenda agenda_day agenda_filtre agenda_month planning planning_mois trame tables tables_search calc carnet
    vaccins bio watch rein grossesse cyp ddi libelles listes revue locations conciliation vaccine_map ordonnance
    protocols protocol_open codex codex_open dispositifs dispositif_open
    finances stats companion script carnets carnets_edit textes graph stup stup_catalogue saisie ordonnancier vigilance destruction scans patient_scans fil registres aide
    explorer explorer_organ classes classes_outside export
    template options about base peaux keys act_picker vitale
    caisse caisses
    goto goto_jump mono_search mono_patient
)

w=${SIZE%x*} h=${SIZE#*x}
for view in "${views[@]}"; do
    # The virtual screen is three times the window wide and the window
    # sits at its left edge: Xvfb parks the pointer in the middle, which
    # on a screen the size of the window is *inside* it, and every shot
    # came back with whatever tooltip happened to be under it.
    layout "$@"
    view="$view" out="$out" w="$w" h="$h" card="$card" \
    xvfb-run -a -s "-screen 0 $((w * 3))x${h}x24" bash -c '
        unset WAYLAND_DISPLAY
        case "$view" in
            drug_edit) export BPM_CADDY_START_VIEW=drug_card BPM_CADDY_DRUG_EDIT=1 ;;
            drug_kin)  export BPM_CADDY_START_VIEW=drug_card BPM_CADDY_KIN=class ;;
            # La carte rejouee, comme dans smoke.sh : sans elle la vue se
            # capture sur le message de lecteur absent, pas sur sa forme.
            vitale)    export BPM_CADDY_START_VIEW=vitale
                       export BPM_CADDY_VITALE_DUMP="$card" ;;
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
