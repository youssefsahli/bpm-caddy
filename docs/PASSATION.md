# Passation — 08/10/2026

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
  par l'empreinte de `DETAILS_REWORDED`.
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
