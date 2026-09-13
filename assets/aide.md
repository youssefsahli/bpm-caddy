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

Le bouton « ? » de la barre du haut, ou `F12`, liste tous les raccourcis
clavier.
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

## Lecture d'ordonnance

Sous la biologie, cinq lectures de la même ordonnance : l'interprétation
des résultats, ce qui n'a pas été demandé depuis trop longtemps, ce que
la clairance du jour change, ce que la grossesse et l'allaitement
changent, et les croisements sur les cytochromes.

**Une forme locale n'est pas lue comme la voie générale.** Un collyre,
une pommade, un gel ou une pulvérisation nasale ne reçoivent ni palier
rénal, ni niveau de grossesse, ni demande d'examen : ces tables sont
rangées par molécule, et la même molécule ne fait pas la même chose
selon la voie. Un collyre à la ciclosporine ne croise rien, un gel au
lithium ne demande pas de lithiémie. Ce qui vaut pour la voie locale
reste écrit sur la fiche du produit, qui est l'endroit où le lire.

Le dernier répond à « ces deux lignes se rencontrent-elles sur une
enzyme ? ». Il **ne connaît que sept cytochromes** : ni la glycoprotéine
P, ni les transporteurs hépatiques, ni les additions d'effets — deux
sédatifs ne se rencontrent sur aucune enzyme et s'additionnent quand
même. Une ordonnance sans croisement n'est pas une ordonnance sans
interaction, et les lignes que la table ne connaît pas sont **nommées**
plutôt que passées sous silence.

Une prodrogue s'y lit à l'envers, et c'est écrit sur la ligne : freiner
l'enzyme qui fabrique le métabolite actif du clopidogrel, de la codéine,
du tramadol, du losartan ou du tamoxifène ne les fait pas s'accumuler,
cela supprime leur effet.

Trois réponses et non deux. Une ligne peut croiser, être **sans voie
connue** — la table la connaît et elle ne passe par aucune des enzymes
suivies, ce qui est la réponse qu'on cherche en se demandant par quoi
remplacer un traitement —, ou être **inconnue de la table**, ce qui
n'est pas la même chose et ne l'innocente pas.

## Peut-on écraser ?

Le bouton « Écraser ? » du dossier imprime une feuille pour l'EHPAD ou
l'infirmière : toute l'ordonnance, ligne par ligne, la conduite pour
chaque forme et l'alternative quand il y en a une.

**Le silence n'est pas une permission.** Un produit que la table ne
connaît pas reçoit « à vérifier », et jamais une ligne absente : une
feuille qui ne montrerait que les interdits se lirait « tout le reste,
oui ».

Trois réponses et non deux, pour ce que la table connaît : oui, non, et
« oui en ouvrant la gélule » — les microgranules s'avalent et ne se
croquent pas, c'est le cas le plus fréquent en gériatrie, et le réduire
à « non » ferait changer une ordonnance qui n'en avait pas besoin.

**C'est la forme qui décide, pas la molécule.** La morphine s'écrase ou
ne s'écrase pas selon la boîte : Moscontin jamais, Skenan en ouvrant la
gélule. Une forme qui ne passe pas par la bouche — injectable,
implantable — reçoit sa ligne elle aussi, pour dire qu'il n'y a rien à
écraser.

Chaque refus dit par quoi remplacer, ou dit qu'il n'y a rien et qu'il
faut appeler le prescripteur. Certains refus protègent celui qui écrase
plutôt que le patient : un cytotoxique, un tératogène — c'est la
poussière qui est le danger, et la feuille le dit.

## La conciliation de sortie

L'onglet « Conciliation » compare l'ordonnance du dossier à celle qu'un
patient rapporte de l'hôpital. On colle la liste de sortie ; chaque
ligne est rapprochée d'une fiche, et l'écran dit ce qui a été arrêté,
changé, ajouté ou remplacé.

Six lectures, et la première passe avant les autres : la ligne que
personne n'a pu rapprocher, parce que personne ne l'a vérifiée. Puis le
remplacement dans la même classe — la divergence dont le patient repart
avec les deux boîtes —, l'arrêt, le changement de posologie, l'ajout, et
ce qui est reconduit sans changement.

La feuille s'imprime à l'attention du prescripteur, avec un cadre laissé
pour sa réponse. Elle ne vaut pas avis médical, et elle l'écrit.

# Croiser une liste

L'écran « Croisement » pose les mêmes questions à une liste qu'on
compose soi-même, sans dossier : depuis le dossier ouvert, ou en tapant
des noms.

Cinq lectures de la même liste : les croisements sur les cytochromes,
le temps que met une exposition déplacée à revenir, la revue
d'ordonnance — doublons, associations, cascades — ce que la clairance
change, si on la tape, et ce que le foie change, au stade qu'on désigne.

**Le foie n'a pas de DFG.** Le rein donne un chiffre qui se lit sur un
compte rendu ; le foie donne un stade — Child-Pugh A, B ou C — qu'un
clinicien attribue à partir de cinq éléments dont deux ne sont pas des
valeurs de laboratoire. Le panneau offre donc trois boutons et non un
champ : un champ inviterait à écrire un chiffre, et il n'y en a pas.

Trois réponses et non deux, comme pour les cytochromes. « On ne sait
pas » est en gris ; « on sait, et il n'y a rien à changer » est écrit en
toutes lettres. L'oxazépam est le cas qui le justifie : sa fiche dit
qu'aucune adaptation n'est nécessaire en insuffisance légère à modérée,
et c'est précisément la benzodiazépine qu'on cherche chez un
cirrhotique. Une liste qui la tairait la rendrait aussi muette qu'un
produit dont personne n'a rien écrit.

Une hépatopathie évolutive n'est pas un stade : les statines y sont
contre-indiquées quel que soit le Child-Pugh, et les ranger sous un
palier dirait la chose à un stade en la taisant aux autres. Leur fiche
le dit là où c'est vrai.

La carte dessine une corde par croisement, de la ligne qui agit vers
celle qui bouge. Sa couleur est l'ordre de lecture, et non une gravité
clinique : le logiciel ne sait ni la dose, ni la durée, ni le terrain.

**Le temps compte autant que le sens.** « Exposition augmentée » ne dit
pas la même chose d'un produit dont la demi-vie est de deux heures et
d'un autre dont elle est de cinquante jours, et un effet peut durer bien
après le produit — l'effet antiplaquettaire du clopidogrel tient sept à
dix jours quand sa demi-vie est de six heures.

# Les entretiens

Un acte porte sa thématique, son état, sa date, sa durée et les
initiales de qui l'a fait. « Tout imprimer » rend la fiche d'entretien,
le bilan et le plan de prise en un seul document.

## L'ordonnance sous protocole

Après un test rapide positif — angine à streptocoque, cystite simple —,
l'écran compose l'ordonnance que le protocole autorise : l'antibiotique,
sa posologie, l'adjuvant et les conseils.

Les molécules, les doses et les durées sont **celles des tables de
conversion** qu'on lit au comptoir, dans le même ordre et les mêmes
mots : la table qu'on consulte et le document qu'on remet ne peuvent pas
dire deux choses différentes, et un test le tient.

Rien n'est jamais sélectionné d'office et toute posologie proposée est
modifiable : l'application propose, le pharmacien décide.

Les adjuvants ne sont pas une liste du programme. Ce sont les fiches
portant l'étiquette voulue, avec leurs propres lignes de posologie :
ajouter un produit, c'est ajouter une fiche.

Aucune mention n'est imprimée d'office ; celles que l'officine veut voir
s'écrivent dans Options › Mentions.

# L'agenda et le planning

L'agenda se lit par jour, par semaine ou par mois. Le filtre **n'efface
pas** : ce qu'il écarte reste dessiné en trait contre la gouttière, et
ce qui chevauche une entrée retenue reste dessiné en entier — sans quoi
le filtre fabriquerait le conflit qu'il devait montrer.

Les quatre lectures parlent **du même moment** : changer de lecture
recadre les autres sur le jour qu'on regardait. Le jour détaillé porte
son liseré dans la grille de la semaine comme dans celle du mois, et
« Imprimer la semaine » sort la semaine affichée.

Le plan de journée porte **un pointillé à l'heure qu'il est**, avec un
point rouge dans la marge. Il ne se dessine que sur la journée
d'aujourd'hui : posé sur un autre jour, il dirait l'heure d'un jour
qu'on ne regarde pas.

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

# Les listes de contrôle

« Listes » tient les suites de choses à cocher : l'ouverture, la
fermeture, le retour de vacances, ce qu'on vérifie avant de délivrer.
Elles s'impriment en A4, une case par ligne, la date et la personne
laissées à remplir.

Ce n'est pas un protocole. Un protocole répond à « que fait-on dans ce
cas-là » et se lit en descendant un arbre ; une liste répond à
« qu'est-ce qu'on n'a pas oublié » et se lit en cochant.

**Rien n'est livré** : une base neuve n'a aucune liste. Une liste
d'ouverture écrite ailleurs qu'à l'officine est une liste que personne
ne coche.

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

Le numéro de délivrance est attribué au moment de l'écriture et n'est
jamais réattribué : une ligne annulée garde le sien, et la suite
continue après lui.

## Les questions que le registre pose

L'onglet « Vigilance » relit le registre et pose trois questions :
rapprochement des délivrances, pluralité de prescripteurs, escalade des
quantités. Ce sont **des questions et non des verdicts** : chacune cite
les lignes qui la motivent, et se vérifie en les relisant.

Ce que le registre ne sait pas, il ne l'invente pas. Une ligne porte un
jour, une quantité, un dossier et un prescripteur — ni la dose
quotidienne, ni la durée prescrite. « Ce traitement aurait dû durer
jusqu'au » ne se calcule donc pas.

La durée maximale de la famille ne sert qu'à **se taire** : au-delà,
deux ordonnances ne peuvent pas se chevaucher, et la question ne se pose
pas. Elle ne sert jamais à déduire un rythme — une ordonnance de sept
jours sous un plafond de vingt-huit passerait pour quatre fois trop
lente, et chaque délivrance légitime deviendrait un signalement. Une
règle qui crie au loup est une règle qu'on éteint.

Sous trois délivrances précédentes, rien n'est dit : il n'y a pas encore
de cadence à laquelle comparer.

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

Les **libellés de l'interface** — ce que l'application dit d'elle-même :
boutons, invites, infobulles — se relisent et se réécrivent dans
« Libellés ». Ils vivent dans un fichier à côté de la configuration,
donc par poste, et une réécriture s'affiche à la prochaine ouverture.

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
