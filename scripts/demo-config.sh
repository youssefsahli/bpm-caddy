#!/usr/bin/env bash
# La configuration d'une officine de démonstration, écrite **une fois**
# pour les deux scripts de capture.
#
# `shot.sh` la posait et `eyeball.sh` non, si bien que les deux outils
# qui existent tous les deux pour *regarder les mêmes vues* n'en
# montraient pas le même état : sans `[pharmacy]`, la grille du planning
# range CL, YS et MB en « personnes que la liste ne connaît pas », et le
# plan de journée n'a aucune plage d'ouverture, donc aucun creux à
# peindre en rouge. La passe large — celle qui trouve les défauts —
# était la moins représentative des deux, et comparer une image de l'une
# à une image de l'autre ne voulait rien dire.
#
# C'est le défaut que ce dépôt nomme partout ailleurs : deux
# constructions d'une même chose finissent toujours par diverger.
#
# Usage : `demo_config <fichier> <échelle> <thème> [monographie]`.
#
# La quatrième est la façon de lire une monographie — « feuille »,
# « dense » ou « lecture ». Elle est là pour la même raison que le
# thème : rien d'autre qu'une capture ne dit ce que change une mise en
# page, et une façon de lire qu'aucun script ne sait produire est une
# façon de lire que personne ne regardera jamais.
#
# `demo_vitale_card <fichier>` écrit la carte Vitale rejouée : le chemin
# du lecteur s'exécute alors en entier sans matériel, sans carte réelle
# et sans l'identité de personne. `smoke.sh` la posait, `eyeball.sh`
# écrivait « ceci n'est pas une carte » — et ne s'en servait même pas,
# faute de brancher `BPM_CADDY_VITALE_DUMP`. La vue Vitale y était donc
# capturée sur son message d'erreur, alors qu'une passe qui regarde
# toutes les vues doit les voir dans leur forme ordinaire.
demo_config() {
    cat > "$1" <<EOF
[ui]
discreet_finances = false
text_scale = $2
theme = "$3"
monograph = "${4:-feuille}"
[pharmacy]
# L'identité, parce qu'une capture d'Options › Officine sur cinq champs
# vides ne montre rien de ce que l'écran fait, et que les documents
# imprimés depuis la base de démonstration se signent avec.
name = "Pharmacie du Centre"
address = "12 place du Marché, 34000 Montpellier"
phone = "04 67 00 00 00"
pharmacist = "Claire Leroy, pharmacien titulaire"
am_number = "341234567"
# L'équipe que la démo sème au planning. Sans elle, la grille range CL,
# YS et MB en « personnes que la liste ne connaît pas » : lisible, mais
# ce n'est pas la forme qu'une officine voit.
operators = [
  { initials = "CL", name = "Claire Leroy", role = "Pharmacien titulaire" },
  { initials = "YS", name = "Yanis Saïd", role = "Pharmacien adjoint" },
  { initials = "MB", name = "Maya Bertrand", role = "Préparatrice" },
]
# Les horaires d'ouverture : sans eux, aucun creux ne se dessine, et la
# bande de couverture du plan de journée n'aurait pas de rouge à montrer.
horaires = [
  { jour = "lundi", de = "09:00", a = "12:30" },
  { jour = "lundi", de = "14:00", a = "19:30" },
  { jour = "mardi", de = "09:00", a = "12:30" },
  { jour = "mardi", de = "14:00", a = "19:30" },
  { jour = "mercredi", de = "09:00", a = "12:30" },
  { jour = "mercredi", de = "14:00", a = "19:30" },
  { jour = "jeudi", de = "09:00", a = "12:30" },
  { jour = "jeudi", de = "14:00", a = "19:30" },
  { jour = "vendredi", de = "09:00", a = "12:30" },
  { jour = "vendredi", de = "14:00", a = "19:30" },
  { jour = "samedi", de = "09:00", a = "12:30" },
]
EOF
}

# `demo_home <dossier>` : les trois dossiers XDG, jetables.
#
# **`XDG_CONFIG_HOME` ne suffit pas.** « À propos » lit la version du
# lanceur dans `dirs::data_dir()`, c'est-à-dire `XDG_DATA_HOME` : sur une
# capture prise sans le rediriger, la page affiche ce que *la machine de
# l'opérateur* a installé — « v0.3.0 » sur une application en 0.200.0 —,
# ce qui n'est ni reproductible ni à montrer.
demo_home() {
    export XDG_CONFIG_HOME="$1"
    export XDG_DATA_HOME="$1/data"
    export XDG_CACHE_HOME="$1/cache"
    mkdir -p "$XDG_DATA_HOME" "$XDG_CACHE_HOME"
}

# `demo_view_env <vue> <carte>` : ce qu'une vue demande **en plus** de
# `BPM_CADDY_START_VIEW`, écrit une fois pour les deux scripts de
# capture.
#
# Quatre des vues que `eyeball.sh` balaie ne sont pas des clés de vue :
# le formulaire d'une fiche et sa liste de voisins sont des drapeaux
# posés par-dessus `drug_card`, le lecteur Vitale demande une carte
# rejouée, et le verrou se montre en *retirant* le mot de passe.
# `eyeball.sh` le savait et `shot.sh` non : `./scripts/shot.sh drug_edit`
# rendait l'écran d'accueil, c'est-à-dire une image de rien, sur l'outil
# qui existe pour le coup d'œil qu'on répète vingt fois pendant qu'on
# corrige une bande. Quatre formes qu'on ne pouvait corriger qu'à
# l'aveugle, ou en relançant la passe entière.
#
# C'est le défaut que ce dépôt nomme partout ailleurs : deux
# constructions d'une même chose finissent toujours par diverger.
demo_view_env() {
    case "$1" in
        drug_edit) export BPM_CADDY_START_VIEW=drug_card BPM_CADDY_DRUG_EDIT=1 ;;
        drug_kin) export BPM_CADDY_START_VIEW=drug_card BPM_CADDY_KIN=class ;;
        # La carte rejouée : sans elle la vue se capture sur le message
        # de lecteur absent, pas sur sa forme.
        vitale)
            export BPM_CADDY_START_VIEW=vitale
            export BPM_CADDY_VITALE_DUMP="$2"
            ;;
        # Le premier écran, et le seul qu'aucune clé de vue ne peut
        # ouvrir — il se montre en retirant le mot de passe.
        verrou) unset BPM_CADDY_PASSWORD ;;
        # « search » est l'accueil : c'est la vue par défaut, et lui
        # poser une clé la ferait passer par le même chemin que les
        # autres, ce qu'elle n'emprunte pas.
        search) ;;
        *) export BPM_CADDY_START_VIEW="$1" ;;
    esac
}

demo_vitale_card() {
    # **Trois bénéficiaires, pas un.** Une carte porte le titulaire et
    # ses ayants droit, et la personne au comptoir n'est pas toujours le
    # titulaire : c'est tout ce que l'écran du lecteur a à montrer, et
    # avec un seul nom il ne le montrait pas. Les trois numéros portent
    # leur clé de contrôle — c'est elle qui les fait reconnaître, jamais
    # leur position.
    printf '\x00DUPONT\x00JEAN\x00155087511600125\x0003081955\x00DUPONT\x00MARIE\x00260027511600221\x0012061962\x00DUPONT\x00LEA\x00105109911602589\x0027042005\x00' > "$1"
}
