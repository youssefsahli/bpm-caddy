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
# Usage : `demo_config <fichier> <échelle> <thème>`.
demo_config() {
    cat > "$1" <<EOF
[ui]
discreet_finances = false
text_scale = $2
theme = "$3"
[pharmacy]
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
