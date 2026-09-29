# Passation — 29/09/2026, 12 h 30

Écrit pour reprendre sans l'ordinateur qui a servi : tout ce qui suit est
dans le dépôt, rien n'est resté en local.

**Versions.** Dernière version publiée : voir `gh release list`. La
0.366.0 (dix fiches : Camzyos, Brukinsa, Exjade, Mekinist, Evrenzo,
Nplate, Disulone, Sunosi, Vumerity, Dysalfa) n'a jamais été étiquetée :
elle est publiée dans la 0.367.0, avec le retrait de Torental, Zoxan et
Zeposia. La 0.359.0 n'a jamais été publiée (son contenu, corrigé, est dans la
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
- Torental, Zoxan et Zeposia : retirés de la base livrée en 0.367.0. Palexia,
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
