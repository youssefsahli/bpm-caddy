# Passation — 10/10/2026

Écrit pour reprendre sans l'ordinateur qui a servi.

**Versions.** 0.389.0 (campagnes paramétrables), puis 0.390.0, 0.391.0
et 0.392.0 (corrections de la relecture) sont **committées sur `main`
et non poussées** : aucune étiquette, rien sur GitHub. 0.389.0, 0.390.0
et 0.391.0 ont passé le smoke dans les quatre formes ; la 0.392.0, qui
les contient toutes, est celle à publier (son smoke est dit dans le
commit qui suit sa version). Pour publier : `git push origin main`,
puis `git tag v0.392.0 && git push origin v0.392.0`.

**Ce qui a été fait le 10/10.**
- Campagnes paramétrables (le travail laissé en cours), relues avant
  d'être committées : cinq défauts corrigés (clé de saison prise par une
  campagne nouvelle, deux lignes de même clé venues de deux postes,
  comparaison qui échouait contre une version plus récente, sujet
  « appel » compté par appel et non par personne, bilan d'une campagne
  sans vaccin). Documents de la campagne en menu sur un écran court.
- **Relecture par domaines**, chaque fois par un agent en lecture seule
  puis vérifiée ici contre le code avant toute correction ; chaque
  correction a son test, qui échoue sur l'ancien code. Le détail est au
  CHANGELOG (0.390.0) et dans les messages de commit. En bref : registre
  (annulation comptée par-dessus un comptage, annulation en double sur
  deux postes, registre laissé en arrière par la copie de la base, lot
  de la boîte précédente gardé au scan), postes de versions différentes
  (effacements jamais appliqués, questions impossibles à régler),
  caisse (dernier comptage choisi par numéro de poste, fond et attendu
  illisibles enregistrés à zéro), honoraires (rang lu à la date de
  saisie), taux horaire, trame (« Remplacer » effaçait les semaines
  passées ; une semaine sur deux reposée hors phase), garde de la veille,
  dates sans année refusées sur sept écrans, table rénale (deux bornes),
  biologie (seuil bas strict, d'accord avec le rein), écrasement
  (« Lasilix Retard » répondu « oui »), carte Vitale (nom du bénéficiaire
  suivant), calendrier et campagnes (dTcaP de la grossesse, campagne
  préparée d'avance, grippe hors saison, COVID trop récent),
  conciliation (deux produits d'une molécule, dose doublée), couverture
  (« Renouveler » recopiait la QSP), carte (noms collés), libellés
  (champ sous le pli).

**À trancher d'abord — un patient atteint les téléphones.** Les
conversations entre officines vivent dans la même table que celles de
l'équipe (`messages`, flux de l'équipe), et ce flux est l'un des quatre
du compagnon. Un message envoyé à une officine avec un patient lié porte
« Patient : NOM Prénom, né(e) le … » dans son texte ; ce qu'une officine
partenaire envoie aussi. Tout cela est copié sur chaque téléphone de
l'équipe — l'application mobile ne fait que le cacher. C'est contraire à
« jamais un patient » (SYNC.md § 7.8). La correction est d'architecture :
router les conversations entre officines sur un flux que le téléphone ne
tient pas (par ligne, ou une table à part), avec une reprise de ce qui a
déjà voyagé — et les téléphones déjà servis gardent ce qu'ils ont reçu.
Liés : un téléphone modifié peut changer une conversation d'équipe en
conversation entre officines (`channel`, `peers`), et le poste de bureau
publie alors l'historique de l'équipe au réseau, sans la confirmation ni
le journal des accès ; un téléphone se déclare lui-même `kind=''` (poste
de bureau) à son arrivée ; un téléphone retiré garde sa clé, faute de
renouvellement des clés. Les trois demandent que le poste qui invite
écrive lui-même ce qu'il a invité.

**Réseau d'officines (relevé, non corrigé).** Les identifiants
`<base>:<numéro>` ne sont pas liés au signataire : un membre peut
prendre d'avance l'identifiant d'un autre (ses événements sont alors
ignorés ailleurs, et ses propres écritures échouent sur l'unicité), et
une version reçue peut créer une fiche portant le nom d'une fiche réelle
sans historique (`versions::decode` ne vérifie pas `card == key(nom)`).
Corriger, c'est préfixer à la réception par l'auteur signé, avec une
reprise des événements déjà rangés. Le retrait d'une rupture, lui, est
corrigé (un retrait ne vaut que de son auteur). Rien ne borne non plus
ce qu'un membre écrit avec le temps, et un fichier `.bpmnet` déposé dans
le dossier d'échange est lu entier.

**Postes joignables (relevé, non corrigé).** Une connexion lente tient
le fil des postes jusqu'à 180 s (le délai de 8 s vaut par lecture) et
une invitation jusqu'à sa fin : un délai court sur la phase non
authentifiée suffirait. Le limiteur de coups compte par adresse et non
par /64. Une erreur dans `auto()` saute le nettoyage (relais, adresse
publiée, UPnP restent ouverts) : un garde à la destruction. Le relais
rend parfois un attendant abandonné au téléphone. Corrigé : le journal
n'est plus réécrit en entier à chaque coup frappé.

**Lanceur (relevé).** Aucune vérification cryptographique du binaire
téléchargé (la confiance est celle du compte GitHub et de TLS) ;
signer les versions (minisign/ed25519, clé publique dans le lanceur)
demande une paire de clés que l'officine éditrice garde. Les actions du
workflow de publication sont citées par étiquette et non par empreinte ;
une version est publiée dès le premier binaire envoyé, avant les autres
plates-formes et sans attendre les tests (passer par un brouillon). Pas
de retour à la version précédente si la nouvelle ne démarre pas.

**Données en clair (relevé).** Les exports CSV (entretiens : nom,
téléphone, naissance ; doses de la saison) s'écrivent dans `exports/` à
côté de la base — donc sur le partage réseau quand la base y est — et
n'en sont jamais retirés. Choisir : un dossier local au poste, une durée
de garde, ou un export chiffré.

**Posologies (relevé).** Les schémas en phases (« 2 cp matin et soir
les 3 premiers jours puis 1 cp matin et soir ») se mêlent dans la
grille ; une demande n'importe où (« 1 cp le matin et si besoin le
soir ») sort toute la ligne de la grille, prise du matin comprise —
voulu, mais une prise systématique se perd ; « 2 x 3/j » reste non
tranché. Protections périodiques : la période court d'anniversaire en
anniversaire depuis la toute première délivrance ; l'autre lecture du
décret la ferait repartir à la délivrance qui suit une interruption —
à confirmer.

**Changement de mot de passe (relevé).** Les trois fichiers sont
rechiffrés l'un après l'autre (pièces, registre, base), sur le fil de
l'écran ; chacun l'est atomiquement, l'ensemble non. Une coupure entre
deux — un gros fichier de pièces sur le partage peut prendre des
minutes, la fenêtre « ne répond plus » et on la tue — laisse une base
qu'aucun des deux mots de passe n'ouvre. À faire hors du fil de l'écran,
avec une marque « rechiffrement en cours » ou un essai des deux clés par
fichier à l'ouverture.

**« Effacer toutes les données » (relevé).** L'agenda, le planning, la
caisse, les protocoles, les prescripteurs, le journal des accès et le
registre restent ; la confirmation dit « TOUTES les données ». Le
registre doit rester (R. 5132-36) ; le reste est à décider, ou la
phrase à corriger.

**Comparaisons restantes (relevé).** Les groupes de collègues
s'écrivent sans comparer ; la fenêtre des valeurs PK écrit champ par
champ sans transaction (un refus au milieu laisse la moitié écrite) ;
les champs de rendez-vous tapés au dossier comparent à la liste relue,
si bien qu'un rendez-vous déplacé ailleurs pendant qu'on tape le sien
est remplacé — le refus, lui, se dit maintenant.

**Ordonnances TROD (relevé).** Grossesse inconnue lue comme « pas
enceinte » (aucune ligne ne le rappelle) ; aucune barrière d'allergie
(l'amoxicilline est proposée quoi que dise le dossier).

**Codex (relevé).** « 1.000 mL » (point des milliers) se lit 1 mL et
multiplie tout par mille ; les formules livrées écrivent « 1000 ».

**Classes à relire (relevé).** Des alias regroupent des molécules que
l'anneau « même classe » de la carte présente alors comme substituts :
« IEC + diurétique » avec Tarka, Coveram et Triplixam (IEC + inhibiteur
calcique) ; CoAprovel et Hyzaar (sartan + diurétique) avec Exforge et
Twynsta (sartan + inhibiteur calcique) ; Iressa (EGFR) avec Sprycel,
Tasigna et Sutent ; Temodal (alkylant) avec Xeloda, Hydréa et Lonsurf.
L'hépatite B et les gliflozines + metformine ont été séparées ; le reste
est un choix de granularité à faire avec la source.

**Messagerie (relevé).** Le non-lu se lit à l'heure du poste qui a
écrit : un message écrit hors ligne et synchronisé plus tard n'est
jamais « non lu », et une horloge en avance fige le compte d'une
conversation. Un brouillon tapé sous un nom part sous le nom choisi
ensuite. Le téléphone ne vérifie pas, dans `messages()` et `save_file`,
qu'une conversation demandée est visible — il ne demande que celles
qu'il a listées, mais l'API le permettrait.

**Feuilles remises (relevé).** Le plan de prise et la fiche de
traitement impriment la posologie en cours de saisie (le champ), pas
celle que la base garde.

**Petits restes.** Bulletins : un champ trop long déborde sans prévenir
(5 pt au minimum, pas de découpe). Console : `patients()` reconstruit
toutes les cartes à chaque appel, si bien qu'une boucle d'appels tourne
une heure avant la limite d'opérations (le bouton d'arrêt marche).

**Décisions qui reviennent à l'officine (relevées, non tranchées).**
- *Changement de poste de référence* : deux postes peuvent numéroter le
  même numéro d'ordonnancier pendant la bascule, et une ligne reçue dont
  le numéro heurte l'index unique est comptée « en double » et
  abandonnée — une délivrance qui manque alors au registre d'un poste.
  Il faut une passation de la numérotation (l'ancien poste cesse avant
  que le nouveau commence) ; c'est un protocole, pas une ligne de code.
  L'index porte aussi sur `(ordo_year, ordo_no)` alors que la suite est
  continue d'une année à l'autre.
- *Effacé ici, modifié là* : un dossier supprimé sur un poste et modifié
  sur un autre reste sur le second, sans son historique (les effacements
  des lignes liées passent), et la question « Vu » ne remet pas les
  postes d'accord. Choisir : rendre la suppression, ou proposer de
  supprimer aussi.
- *Deux postes invités en même temps* peuvent prendre le même numéro de
  poste, donc le même bloc d'identifiants. Rare ; à fermer par le poste
  qui invite (il réserve le numéro), pas par celui qui rejoint.
- *Blocs d'identifiants* : rien ne prévient quand un poste approche la
  fin de son bloc (un fondateur dont le plus grand numéro de dossier est
  199 990 n'a que neuf numéros d'avance).
- *Table rénale* : six lignes écrivent « entre X et Y » avec Y égal à
  leur palier strict (édoxaban 15–50, ciprofloxacine 30–60,
  métoclopramide 15–60, cotrimoxazole 15–30, fondaparinux 20–50,
  dabigatran 30–50) ; à Y tout juste, elles se taisent. Trancher contre
  chaque RCP, borne par borne — deux autres l'ont été (céfpodoxime,
  Tarka) parce que leur source citée le disait.
- *Conciliation* : une association prise par le préfixe de l'un de ses
  composants (« Dafalgan codéine » lu comme Dafalgan, la codéine perdue ;
  « MicardisPlus » comme Micardis) ; « 1 g » contre « 1000 mg » lu comme
  un changement de dose. Une règle de mots pour l'un, une normalisation
  des unités pour l'autre — à écrire avec des exemples réels.
- *Couverture* : les unités qui restent d'une boîte (28 comprimés à
  trois par jour) ne passent pas à la suivante ; l'erreur va du côté
  prudent (la boîte suivante paraît due un jour trop tôt).
- *Carnet* : une dose saisie au nom (« Efluelda ») sans choisir la
  suggestion reste sans code et ne règle jamais un rappel.
- *Carte Vitale* : les ayants droit qui portent le NIR de l'ouvrant
  droit sont dédoublonnés avec lui.
- *dTcaP* : un rappel fait avant l'âge du rendez-vous (64 ans, après une
  plaie) ne compte pas pour celui de 65 ans ; à vérifier contre le HCSP.
- *TROD angine* : la table dit « enfant à partir de 3 ans », l'ordonnance
  ne propose rien avant 10 ans — cohérent avec le périmètre du
  pharmacien, à confirmer.

**Pièges de la machine.** Inchangés (voir plus bas). Le smoke se lance
depuis un `git worktree` du commit à publier ; le retirer après
(`git worktree remove`), il pèse 4 Go.

---

# Passation précédente — 08/10/2026

Écrit pour reprendre sans l'ordinateur qui a servi : tout ce qui suit est
dans le dépôt, rien n'est resté en local.

**Versions.** Publiées (étiquette poussée, smoke passé dans les quatre
formes) : 0.367.0, 0.369.0, 0.372.0, 0.376.0, 0.377.0, 0.378.0,
0.379.0, 0.380.0, 0.381.0, 0.382.0, 0.383.0, 0.384.0, 0.385.0, 0.386.0 et 0.387.0. Les versions
intermédiaires (0.368, 0.370, 0.371, 0.373 à 0.375) ne sont pas
étiquetées : leur contenu est dans la 0.376.0. Ce qui a suivi sur `main`
(section Unreleased du CHANGELOG) attend la prochaine version.

**Ce qui a été fait le 08/10** (détail au CHANGELOG, raisons dans
docs/ARCHITECTURE.md § « La campagne et Mesures et conseils », contenu
dans docs/CONTENU.md) :
- Onglet « Campagne » : rappels de la saison (dus par le calendrier, puis
  traitements évocateurs avant 65 ans), issue de l'appel, liste et bilan
  imprimables, doses par vaccin, lots reçus et stock lu au carnet ;
  carnet qui propose le lot et signale ce que la dose appelle ;
  calendrier 2026 (pneumocoque à 65 ans, HPV jusqu'à 26 ans, ACWY de
  l'adolescent, délai du COVID) ; table « Vaccination à l'officine »
  relue ; six fiches de vaccins et d'anticorps, neuf fiches relues.
- « Mesures et conseils » : compression (mesures, grilles de l'officine,
  renouvellement), nutrition orale (dénutrition HAS, critères LPP, plan
  de CNO, dix-huit produits), protections périodiques réutilisables
  (droits, période, codes individuels) ; fiches imprimées réécrivables.
- Relecture de la rédaction de tout le contenu livré (environ 3 300
  phrases) au registre professionnel de santé, refusée dès qu'un nombre
  change ; elle atteint les bases existantes par les empreintes de
  `src/shipped.rs`.
- Campagne, suite : « Rendez-vous » sur une ligne à rappeler (acte de
  vaccination planifié, que le carnet réalise le jour venu), liste des
  rendez-vous jour par jour ; seconde dose de grippe signalée au carnet.
- « Couverture des délivrances » (outil, et action du dossier) : jours
  couverts par boîte, historique des délivrances sur une rangée (report
  d'une délivrance anticipée, jours sans traitement, boîte en retard),
  échelle de temps zoomable avec jour pointé ou épinglé, alignement des
  renouvellements (quantité en unités et en boîtes), feuille imprimée ;
  unités par boîte retenues au dossier (`patient_drugs.box_units`).
- Culottes menstruelles : tour de bassin et stature à la délivrance.
- Déverrouillage plus rapide (fichiers annexes ouverts en parallèle),
  `scripts/frames.sh` pour le coût des images.

**Décisions qui reviennent à l'officine.**
- Prise en charge des CNO avant 70 ans : **tranché**. L'arrêté du 7 mai
  2019 (JORF, article JORFARTI000038456511) retient avant 70 ans la perte
  de poids (≥ 5 % en 1 mois ou ≥ 10 % en 6 mois) ou l'IMC ≤ 18,5 (hors
  maigreur constitutionnelle) ; l'albuminémie (< 35 g/L), l'IMC ≤ 21 et
  le MNA ≤ 17 ne valent qu'à partir de 70 ans. `nutrition::assess` lit
  exactement ces critères.
- Capvaxive : **vérifié** — avis de la HAS du 30 juillet 2026 (révision
  de la stratégie contre les pneumocoques) : Capvaxive préférentiellement
  au VPC20 chez l'adulte à risque et à partir de 65 ans. Le catalogue, le
  calendrier et la table le disent maintenant. Comirnaty : la
  contre-indication après myocardite vient de la liste française de
  juillet 2021 (le RCP la traite en mise en garde, rubrique 4.4) ; la
  fiche le dit maintenant, et les bases existantes reçoivent la phrase
  par l'empreinte de `DETAILS_REWORDED`, au déverrouillage du poste de
  référence, une fois par version.
- Kerendia : **complété** — la remarque donne maintenant les paliers du
  RCP (instauration sous 5, passage à 20 mg à 4,8 ou moins, suspension
  au-dessus de 5,5, reprise à 10 mg à 5 ou moins) ; les bases existantes
  la reçoivent par l'empreinte de `POSOLOGIES_REWORDED`.
- Les anciennes fiches Vaxigrip Tetra et Prevenar 13 portent un statut
  (retrait, n'est plus recommandé chez l'adulte) mais restent : les
  retirer ou non.

**Pièges de la machine.** Pas de Xvfb ni de sudo sur ce poste : les
scripts de capture tournent avec `scripts/headless/xvfb-run` (KWin
sans écran et Xwayland en mode racine) : `PATH="$PWD/scripts/headless:$PATH"`. CI : chaque poussée lance un smoke de
quarante-cinq minutes ; annuler les exécutions dépassées quand plusieurs
s'empilent.

---

# Passation précédente — 29/09/2026, 12 h 30

Écrit pour reprendre sans l'ordinateur qui a servi : tout ce qui suit est
dans le dépôt, rien n'est resté en local.

**Versions.** Dernière version publiée : voir `gh release list`. La
0.366.0 (dix fiches : Camzyos, Brukinsa, Exjade, Mekinist, Evrenzo,
Nplate, Disulone, Sunosi, Vumerity, Dysalfa) est committée sur `main`,
toutes les portes passées (fmt, clippy, tests, deux `cargo check`,
couverture), **sans le smoke** faute de temps : lancer
`./scripts/smoke.sh`, puis `git tag v0.366.0 && git push origin v0.366.0`.
La 0.359.0 n'a jamais été publiée (son contenu, corrigé, est dans la
0.360.0).

**Ce qui a été fait la nuit du 28 au 29** (le détail est au CHANGELOG,
les raisons dans docs/ARCHITECTURE.md et docs/CONTENU.md) : cinq
transporteurs dans la lecture des cytochromes (P-gp, OATP1B1, BCRP,
OCT2, MATE1), le tocilizumab et le sarilumab qui rendent les CYP ;
59 cascades, relues par des pharmacologues et leurs lignes « RCP de »
vérifiées contre les RCP ; « Sur une même cascade » au croisement, au
dossier, sur la carte et à la barre de comptoir ; la section
« Cytochromes et transporteurs » des monographies (écran et PDF) ;
sommaire du croisement ; filtre des cascades ; 44 fiches relues ou
ajoutées d'après leur RCP.

**Décisions qui reviennent à l'officine.**
- Torental, Zoxan et Zeposia ne figurent plus à la BDPM ; Palexia,
  Intuniv et Mayzent ne sont pas commercialisés en France (leur statut
  le dit) ; Bydureon est abrogé. Garder, renommer vers le générique ou
  retirer.
- Exjade : nombreux génériques du déférasirox — nommer la fiche par la
  DCI ? Disulone : classe « sulfone — lèpre et dermatoses » rangée en
  infectio (derm possible).
- Brukinsa 160 mg comprimé et Mekinist solution buvable : non vérifiés
  en ville ; les lignes d'écrasement ne parlent que des formes vues.

**Limites connues du modèle des cascades** (écrites en commentaire sous
chaque figure) : `adaptation` ne sait pas dessiner une poussée suivie
d'une désensibilisation (agonistes de la GnRH) ni une atrophie
(surrénale privée d'ACTH) ; pas de seuil (le GLP-1 glucodépendant) ; pas
de facteur qui n'agit qu'en présence d'un autre (bradycardie et QT).

**Pistes laissées ouvertes.**
- Odefsey et Genvoya : **fait** — une ligne par spécialité dans `cyp.rs`,
  avant celles de leurs composants, qui garde le rôle de substrat de la
  P-gp du ténofovir alafénamide.
- La carte pharmacologique : **fait** — la raison « même cascade »
  (`graph::Why::Cascade`) fait entrer un voisin dans l'anneau des
  interactions ; l'index de la carte sait les cascades de chaque fiche.
- OAT1/OAT3 (méthotrexate sous AINS) : recherché le 08/10 — ni l'ANSM,
  ni les synthèses consultées ne nomment le transporteur ; elles parlent
  de sécrétion tubulaire diminuée (probénécide, pénicillines, IPP) et,
  pour les AINS, d'une baisse du débit de filtration. La table reste sans
  ligne OAT tant que la rubrique 4.5 d'un RCP ne le nomme pas.

**Méthode de contenu qui a marché** (outils dans `scripts/contenu/`) :
des agents rédigent chaque fiche ou cascade dans un fichier à sections
(`BRIEF-fiches.md`, `BRIEF-cascades.md`, exemple `exemple-palexia.rs`) en
téléchargeant le RCP (BDPM `affichageDoc.php?specid=<CIS>&typedoc=R`, ou
l'EPAR français de l'EMA) ; `insert.py <dépôt> fichiers…` les colle en
fin de table ; les tests nomment ensuite chaque correction. Une seconde
relecture, indépendante, contre le RCP a trouvé des erreurs à chaque
fois : la garder.

**Pièges de la machine.** `target/debug/deps` garde chaque ancien
binaire de test (`bpm_caddy-<hash>`, des centaines de Mo) : 65 Go en une
journée ; supprimer ceux de plus de deux heures, et
`target/llvm-cov-target` après chaque couverture. Le smoke se lance
depuis un `git worktree` du commit à publier pour continuer à
travailler à côté. `pkill -f <motif>` tue son propre shell quand le
motif figure dans la commande.
