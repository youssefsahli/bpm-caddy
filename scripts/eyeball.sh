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
# `vierge=1` est la seule clé qui ne va pas dans `layout.toml` : elle
# saute le semis de démonstration. L'application sème alors son contenu
# livré — les 862 fiches, les préparations, les listes de contrôle — et
# rien d'autre : ni dossier, ni entretien, ni ligne de registre, ni
# comptage de caisse. C'est **le premier écran qu'une officine voit**,
# et aucun script de capture ne l'avait jamais montré : les états vides
# ne se relisent nulle part ailleurs, et c'est là qu'une bande mal
# provisionnée tranche la seule phrase du volet.
#
#   ./scripts/eyeball.sh /tmp/vierge 1024x700 1.25 motif vierge=1
#
# Requires xvfb-run and ImageMagick. Run from the repo root.
set -euo pipefail
# La même configuration de démonstration que `shot.sh` : les deux
# scripts regardent les mêmes vues, ils doivent en montrer le même état.
. "$(dirname "$0")/demo-config.sh"

# `mono=` : la façon de lire une monographie, comme dans `shot.sh` — la
# seconde clé qui va dans config.toml et non dans `layout.toml`.
MONO=feuille
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
fresh=
layout() {
    : > "$tmp/config/bpm-caddy/layout.toml"
    for kv in "$@"; do
        # `vierge=` ne décrit pas la forme du plan de travail mais l'état
        # de la base : elle est retenue ici plutôt qu'écrite dans
        # `layout.toml`, comme `theme=` l'est dans `shot.sh`.
        if [ "${kv%%=*}" = vierge ]; then
            fresh=${kv#*=}
        elif [ "${kv%%=*}" = mono ]; then
            MONO=${kv#*=}
        else
            printf '%s = %s\n' "${kv%%=*}" "${kv#*=}" >> "$tmp/config/bpm-caddy/layout.toml"
        fi
    done
}
layout "$@"
demo_config "$tmp/config/bpm-caddy/config.toml" "$SCALE" "$THEME" "$MONO"
demo_home "$tmp/config"
export BPM_CADDY_WINDOW="$SIZE"

if [ -z "$fresh" ]; then
    BPM_CADDY_SEED_DB="$BPM_CADDY_DB" cargo test seed_demo >/dev/null
fi
cargo build
# Le binaire figé, comme dans `smoke.sh` : une passe dure un quart
# d'heure, et un `cargo build` fait entre-temps changerait le code
# qu'elle regarde au milieu des captures.
bin="$tmp/bpm-caddy"
cp ./target/debug/bpm-caddy "$bin" || exit 1
export bin

card="$tmp/vitale.bin"
demo_vitale_card "$card"

views=(
    verrou search dashboard patient patient_edit patient_new drugs drug_card drug_edit drug_kin
    agenda agenda_day agenda_filtre agenda_month planning planning_mois trame tables tables_search calc carnet
    vaccins bio watch rein grossesse age cyp ddi ddi_crush libelles listes revue locations conciliation vaccine_map ordonnance ordonnance_lignes vaccins_grossesse vaccins_catalogue ruptures reseau versions postes postes_seul connexions
    protocols protocol_open codex codex_open dispositifs dispositif_open
    finances stats companion companion_poso companion_conseils companion_soins companion_dossier companion_vide companion_doublon companion_boite script carnets carnets_edit textes graph graph_zoom graph_wide graph_ordonnance graph_filtre stup stup_catalogue saisie ordonnancier vigilance destruction scans patient_scans patient_dose fil registres regles aide
    explorer explorer_organ classes classes_outside export
    template options about base peaux keys keys_outils nouveautes messages connexions_carte act_picker vitale
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
    # **Chaque vue sur sa propre copie de la base.** « postes » fonde un
    # groupe en s'ouvrant : sans copie, « postes_seul » — un poste hors
    # groupe à qui l'on montre la consigne — se capturait sur la base
    # que la vue d'avant venait de grouper, et montrait la liste des
    # postes à la place de ce qu'elle existe pour montrer.
    # La base vierge, elle, est semée par la première vue qui l'ouvre,
    # puis photographiée (`$tmp/vierge`) : les suivantes en reçoivent
    # une copie. Sans la photo, « patient_new » laissait un dossier et
    # « messages » une conversation à toutes les vues d'après — et
    # l'écran du premier lancement montrait un patient.
    base="$tmp/demo.db"
    src="$tmp"
    if [ -n "$fresh" ]; then
        src="$tmp/vierge"
    fi
    if [ -z "$fresh" ] || [ -e "$tmp/vierge/demo.db" ]; then
        rm -rf "$tmp/vue" && mkdir -p "$tmp/vue"
        for f in "$src"/demo*; do [ -e "$f" ] && cp "$f" "$tmp/vue/"; done
        base="$tmp/vue/demo.db"
    fi
    # Fonder un groupe écrit le journal des postes — plusieurs secondes
    # en debug : à trois, l'image était noire.
    case "$view" in postes | postes_seul | connexions* | reseau) wait=12 ;; *) wait=3 ;; esac
    # La vue qui sème la base vierge a le temps de finir : c'est sa base
    # que toutes les suivantes recevront.
    if [ -n "$fresh" ] && [ ! -e "$tmp/vierge/demo.db" ]; then wait=15; fi
    BPM_CADDY_DB="$base" wait="$wait" \
    view="$view" out="$out" w="$w" h="$h" card="$card" \
    xvfb-run -a -s "-screen 0 $((w * 3))x${h}x24" bash -c '
        unset WAYLAND_DISPLAY
        . "'"$(cd "$(dirname "$0")" && pwd)"'/demo-config.sh"
        # Les quatre vues qui demandent autre chose qu une cle de vue :
        # ecrites une fois, dans `demo-config.sh`, et lues par les deux
        # scripts de capture. `shot.sh` ne les connaissait pas, et rendait
        # l ecran d accueil pour chacune des quatre.
        demo_view_env "$view" "$card"
        "$bin" &
        app=$!
        sleep "$wait"
        import -window root +repage -crop "${w}x${h}+0+0" +repage \
            "$out/$view.png" 2>/dev/null
        kill "$app" 2>/dev/null
        wait "$app" 2>/dev/null || true
    '
    # La première base semée, gardée telle quelle pour les suivantes.
    if [ -n "$fresh" ] && [ ! -e "$tmp/vierge/demo.db" ] && [ -e "$tmp/demo.db" ]; then
        mkdir -p "$tmp/vierge"
        for f in "$tmp"/demo*; do [ -e "$f" ] && cp "$f" "$tmp/vierge/"; done
    fi
    echo "  $out/$view.png"
done
echo
echo "Regardez-les : $out"
