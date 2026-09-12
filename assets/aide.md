# BPM-Caddy

Le suivi des entretiens pharmaceutiques au comptoir : les dossiers, les
actes, l'agenda de l'équipe, le registre des stupéfiants, la caisse — et
ce qui s'imprime au nom de l'officine.

Tout est local. La base est chiffrée et ne sort pas du poste ; rien n'est
envoyé nulle part. La seule requête réseau de l'application est la
recherche d'une mise à jour, et elle ne part que si on presse le bouton
qui la demande.

# Le plan de travail

Trois volets autour d'un cahier d'onglets.

- À gauche, la navigation : le mois, les prochains rendez-vous, la liste
  qu'on cherche.
- Au centre, la vue ouverte. Les onglets du haut en gardent plusieurs
  ouvertes à la fois.
- À droite, ce volet-ci : les notes d'équipe, le carnet du jour, les
  notes personnelles, et cette aide.

Les deux volets latéraux se ferment et se redimensionnent ; leur largeur
et la vue ouverte sont retrouvées à la session suivante.

Le bouton « ? » de la barre du haut liste tous les raccourcis clavier.
C'est la liste à jour : elle est tenue par l'application elle-même, et
non recopiée ici.

# Trouver quelque chose

« Aller à… » cherche partout d'un seul champ : dossiers, fiches, tables
de conversion, préparations, protocoles, dispositifs, registre, carnets
de suivi, scripts. Tapez ce dont vous vous souvenez — le nom, un bout du
nom, ou les initiales.

La recherche ignore la casse et les accents, et accepte les lettres dans
l'ordre sans qu'elles se suivent : « jndp » retrouve Jean Dupont.

Dans les monographies, « Dans le texte… » cherche les **mots** des
fiches et non leur titre : il rend chaque fiche qui dit le mot, avec la
phrase qui le porte.

# Le dossier

Le bandeau porte l'identité et les traitements ; en dessous, les actes,
le journal, la biologie, les vaccins et les pièces scannées.

Les dates s'écrivent court : `230826` ou `2308` suffisent, et la case
affiche `23/08/2026`. Les heures aussi : `9`, `930`, `9h30`.

Ce qui est partagé entre les postes — l'identité, les états d'acte, les
rendez-vous — est écrit contre ce que l'écran affichait. Si un autre
poste a modifié la même ligne entre-temps, l'écriture est refusée et
l'écran se recharge plutôt que d'écraser le travail de quelqu'un.

# Les entretiens

Un acte porte sa thématique, son état, sa date, sa durée et les
initiales de qui l'a fait. « Tout imprimer » rend la fiche d'entretien,
le bilan et le plan de prise en un seul document.

Rien n'est jamais sélectionné d'office et toute posologie proposée est
modifiable : l'application propose, le pharmacien décide.

# L'agenda et le planning

L'agenda se lit par jour, par semaine ou par mois. Le filtre **n'efface
pas** : ce qu'il écarte reste dessiné en trait contre la gouttière, et
ce qui chevauche une entrée retenue reste dessiné en entier — sans quoi
le filtre fabriquerait le conflit qu'il devait montrer.

« Planning » est la quatrième lecture : l'équipe, une ligne par
personne, sept colonnes de jours. Le total d'un jour passe au rouge
quand l'officine est ouverte et que personne n'est inscrit.

## Les rythmes

Un poste revient selon un rythme : ce jour-là, tous les jours jusqu'à
une date, chaque semaine, les semaines paires, les semaines impaires,
une semaine sur deux, sur trois, sur quatre.

**« Les semaines paires » n'est pas « une semaine sur deux ».** L'une se
lit sur le calendrier, l'autre se compte depuis le jour posé. Elles
tombent ensemble pendant des années et se séparent pour toujours au
premier passage d'une année de 53 semaines.

Un congé se pose comme une plage : « tous les jours », du 12 au 26. Une
date de fin est alors obligatoire.

## La trame de la semaine

« Trame… » écrit la semaine entière d'une personne en une fois. Elle
s'ouvre sur la trame déjà posée : c'est un écran pour corriger, pas
seulement pour ajouter.

Une journée coupée s'y écrit en **deux postes** — 9 h – 12 h 30 puis
14 h – 19 h 30 — et non en un poste à longue pause : une pause n'a pas
d'heure, si bien que la bande de couverture compterait la personne au
comptoir pendant sa coupure.

Sur une alternance, deux onglets : la semaine paire et l'impaire. Une
journée identique sur les deux est écrite une fois, hebdomadaire.

Ce que deux semaines de sept jours ne portent pas n'est pas approximé :
l'écran le dit et ne propose pas de remplacer ce qu'il ne montre pas.

## Corriger un jour sans effacer la série

« Ce jour seulement » fait porter « Modifier » et « Supprimer » sur une
seule occurrence. La trame reste écrite, une ligne la contredit ce
jour-là, et la semaine dit la vérité.

# Le registre des stupéfiants

Il vit dans son propre fichier, chiffré comme la base, et **il ne
s'efface pas**. Une ligne fautive reste écrite ; une seconde ligne la
nomme et défait exactement ce qu'elle avait fait.

Le solde est deux nombres et non un : ce qui est délivrable, et ce qu'un
patient a rapporté et qui attend sa destruction.

Une case vide n'est pas un zéro. Sur une feuille de comptage, seuls les
produits qu'on a réellement comptés sont inscrits.

Une ligne porte le **numéro de dossier** et jamais le nom : un registre
s'imprime et se laisse sur un comptoir.

# La caisse

Le comptage se fait en centimes entiers, jamais en flottants. L'écart
avec la recette attendue est **énoncé, jamais résorbé** : le compte
n'est pas recalculé depuis ce qui était attendu.

Sans recette attendue, il n'y a pas d'écart et la ligne reste vide.

Un soir recompté est une **seconde ligne**, pas une correction : le mois
garde le dernier comptage de chaque soir et montre les autres barrés.

# Ce qui s'imprime

Chaque document imprimable a un modèle éditable — Options › Modèles. Un
modèle est du Typst ; les `{{MARQUEURS}}` qu'il accepte sont listés dans
l'éditeur, et l'aperçu passe par la même fonction que l'impression.

Les phrases qui partent sur du papier au nom de l'officine se
réécrivent, dans la vue qui montre le document ou dans l'écran « Textes
imprimés ». Ce qui est réécrit est rangé dans la base et vaut donc pour
tous les postes.

Une réécriture se souvient de la phrase qu'elle remplaçait. Si la phrase
livrée change, la réécriture est montrée à relire plutôt que posée sur
une autre phrase.

# La base

Un seul fichier chiffré, que plusieurs postes peuvent partager. Le
registre des stupéfiants et les pièces scannées ont chacun le leur, à
côté.

Les sauvegardes sont quotidiennes et gardées en nombre fixé ; « Copier
la base… » emporte les trois fichiers. Changer le mot de passe rechiffre
les trois.

Supprimer des pièces ne rend pas la place : seul « Compacter » le fait.

Ce qui appartient à l'officine — son identité, l'équipe, les horaires —
est rangé dans la base et vaut pour tous les postes. Le reste de
`config.toml` appartient au poste.

# La console

Un endroit pour poser à la base une question que personne n'a prévue, en
quelques lignes. Le détail de ce qu'elle sait lire est plus bas, dans
« L'API de la console ».
