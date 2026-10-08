# Passation — 08/10/2026

Écrit pour reprendre sans l'ordinateur qui a servi : tout ce qui suit est
dans le dépôt, rien n'est resté en local.

**Versions.** Publiées (étiquette poussée, smoke passé dans les quatre
formes) : 0.367.0, 0.369.0, 0.372.0. La 0.376.0 est sur `main`, toutes
les portes passées (fmt, clippy avec Rust 1.99, tests, deux `cargo
check`, couverture), smoke en cours au moment d'écrire : quand il passe,
`git tag v0.376.0 <commit> && git push origin v0.376.0`. Les versions
intermédiaires (0.368, 0.370, 0.371, 0.373 à 0.375) ne sont pas
étiquetées : leur contenu est dans la 0.376.0.

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
- Déverrouillage plus rapide (fichiers annexes ouverts en parallèle),
  `scripts/frames.sh` pour le coût des images.

**Décisions qui reviennent à l'officine.**
- Prise en charge des CNO avant 70 ans : le logiciel lit les critères du
  mémo de l'Assurance Maladie (perte de poids, IMC ≤ 18,5) ; une relecture
  a évoqué une albuminémie < 30 g/L, non retrouvée dans le mémo. À
  trancher sur le texte de l'arrêté du 7 mai 2019.
- Capvaxive : la fiche dit une préférence HAS de juillet 2026 qui n'a pas
  été vérifiée ; Comirnaty : la contre-indication après myocardite n'est
  pas au 4.3 du RCP. Les deux phrases sont restées.
- Kerendia : la remarque dit « pas d'instauration au-dessus de 5 ;
  suspension au-dessus de 5,5 » (RCP) ; l'augmentation de dose demande
  une kaliémie ≤ 4,8, non écrite.
- Les anciennes fiches Vaxigrip Tetra et Prevenar 13 portent un statut
  (retrait, n'est plus recommandé chez l'adulte) mais restent : les
  retirer ou non.

**Pièges de la machine.** Pas de Xvfb ni de sudo sur ce poste : les
scripts de capture tournent avec un `xvfb-run` de remplacement (KWin
sans écran et Xwayland en mode racine), à remettre en tête du `PATH` ;
voir l'historique de la session. CI : chaque poussée lance un smoke de
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
- Odefsey et Genvoya perdent le rôle de substrat de la P-gp du ténofovir
  alafénamide (la ligne de la rilpivirine ou du cobicistat parle
  d'abord) : une ligne par spécialité réglerait cela.
- La carte pharmacologique n'a pas de raison « même cascade » : ses
  raisons n'annotent que les nœuds qu'elle trace déjà.
- OAT1/OAT3 (méthotrexate sous AINS) : les fiches ne nomment pas le
  transporteur ; les compléter d'après le RCP d'abord, la table ensuite.

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
