# Passation — 10/10/2026

Écrit pour reprendre sans l'ordinateur qui a servi.

**Versions.** 0.389.0 (campagnes paramétrables), puis 0.390.0 à 0.397.0
(corrections de la relecture) ont passé le smoke dans les quatre formes.
La 0.397.0 est publiée (étiquette `v0.397.0` poussée le 11/10, versions
construites par la CI) ; la 0.398.0 (relectures du 11 et du 12/10) a passé
smoke et CI, son étiquette reste à poser ; la 0.399.0 (copies de la base)
a passé son smoke (fa33b51) ; la 0.400.0 (relecture et frappe, registre,
actes) a passé son smoke dans les quatre formes (bfffa2d) ; la 0.401.0
(versions, agenda, campagne, biologie) a passé son smoke (43d0bd8) ; la
0.402.0 (automesure, tables, ruptures, console, dossiers) a passé son
smoke dans les quatre formes (5012a7a) ; la 0.403.0 (messagerie,
téléphone, codex, dispositifs, tableau de bord, pièces, caisse,
nutrition) a passé son smoke dans les quatre formes (bac8fce), et la
relecture des régressions n'y a rien trouvé ; **les 0.404.0 à 0.409.0 ne se publient pas** — la relecture de leurs
corrections y a trouvé des régressions : la conciliation lisait
« arrêtée » une molécule écrite sans son sel (Dépakine contre
« Valproate »), un enregistrement de textes effaçait les réécritures
périmées de tout le document, la conservation du journal ne redescendait
jamais, une suppression refusée vidait quand même le dossier, le lot
d'une boîte lue perdu, le code d'une dose corrigée effacé, un DFG
arrondi qui taisait une contre-indication ; corrigées dans la 0.410.0 —
chaque correction relue commit par commit, puis l'ensemble depuis la
0.403.0 relu d'un bloc (ni chute, ni registre abîmé, ni lecture
clinique fausse ; les deux pertes silencieuses trouvées alors, le lot
tapé d'un autre produit et les adresses des réécritures rénales, sont
corrigées) — a passé son smoke dans les quatre formes et une passe de
fuzz sur toutes les vues (d33dbe5) ; la 0.411.0 (numéros
d'ordonnancier tapés depuis le logiciel de l'officine, téléphone, aide —
chaque commit relu) a passé son smoke dans les quatre formes (4e6fa5b) ;
la 0.412.0 (saisie groupée de délivrance : numéro manquant ou tapé deux
fois dit sur sa ligne, « Numéroter à la suite », quantité au-delà du
solde — relue, corrigée, relue) a passé son smoke dans les quatre formes
(bdceec1) ; la 0.413.0 (« En ligne » : chercher et prendre ce que les
officines du réseau partagent ; dispositifs, cascades, listes et phrases
imprimées voyagent, les deux dernières sur demande et jamais appliquées
d'elles-mêmes — chaque commit relu, quatre relectures) a passé son smoke
dans les quatre formes (810cb49) ; la 0.414.0 (dispositifs, listes et
cascades renommés suivis sous une identité figée, unique — relue,
corrigée) a passé son smoke dans les quatre formes (db9d2da) ; la
0.415.0 (« En ligne » compte ce qui a bougé, met le nouveau en tête,
filtre ce qui manque ici, parcourt une officine — deux relectures,
corrigées) a passé son smoke dans les quatre formes (634ea4b) ; la
0.416.0 (la carte des officines : un répertoire qu'une officine
héberge, sans appairage — cinq relectures dont deux de sécurité,
corrigées) a passé son smoke dans les quatre formes (309fddd) ; la
0.417.0 (le FINESS déclaré lu au registre public livré, tout le
répertoire cherché d'un geste, la liste de la carte filtrée et triée par
distance — chaque commit relu, quatre relectures corrigées) a passé son
smoke dans les quatre formes (d0d8a48) ; la 0.418.0 (la carte
s'agrandit et se déplace, Options › Officine retrouve l'officine au
registre FINESS, la carte suggère son FINESS — chaque commit relu) a
passé son smoke dans les quatre formes (539df12) ; la 0.419.0 (le
répertoire en IPv6, sans transfert de port, et ses bornes par abonné et
par réseau — trois relectures de sécurité, corrigées) a passé son smoke
dans les quatre formes (2b58429) ; la 0.420.0 (l'onglet de la carte
compte les officines apparues, « En ligne » déplie une entrée en entier
— relus, corrigés) a passé son smoke dans les quatre formes (da6c454) ;
la 0.421.0 (« En ligne » filtre « Différentes d'ici » et prend la version
d'une officine du répertoire — deux relectures, corrigées) a passé son
smoke dans les quatre formes (d0d0162), **étiquetée et publiée le
13/10** ; la 0.422.0 (ce que l'officine publie se voit avant de
paraître, une entrée supprimée ne se publie plus, le carnet avertit d'un
lot périmé ou d'un autre vaccin — relus, corrigés) a passé son smoke
dans les quatre formes (0e8b170), **étiquetée et publiée le 13/10** ;
la 0.423.0 (un résultat borné par le laboratoire se lit et s'écrit comme
il a été rendu, le rein ne conclut que ce que la borne permet — trois
relectures cliniques, corrigées) a passé son smoke dans les quatre
formes (d568444), étiquetée le 13/10 ; la 0.424.0 (une valeur bornée
qui ne tranche pas une règle de biologie ou un niveau s'annonce « à
vérifier » partout où la biologie se lit — deux relectures cliniques,
corrigées) a passé son smoke dans les quatre formes (fb56c64),
étiquetée le 13/10 ; la 0.425.0 (une invitation se ferme dans la seconde,
les gestes des postes qui ne se défont pas en deux clics — relus,
corrigés) a passé son smoke dans les quatre formes (e9e5884), étiquetée
le 13/10 ; la 0.426.0 (dossier d'échange de toute taille,
bulletin qui dit ce qui déborde, console bornée en lignes — relus,
corrigés) a passé son smoke dans les quatre formes (b6519a7), étiquetée
le 13/10 ; la 0.427.0 (les notes d'équipe dans la base chiffrée —
trois relectures, corrigées) a passé son smoke dans les quatre formes
(23bec61), étiquetée le 13/10 ; **la 0.428.0 ne se publie pas** (le fil
sautait, des options rendues après un verrouillage écrasaient
l'officine — corrigé dans la suivante) ; la 0.429.0 (la feuille de
l'infirmier(ère) — relue, corrigée) a passé son smoke dans les quatre
formes (afd99c9), étiquetée le 13/10 ; la 0.430.0 (copies par étapes
bornées, changement de mot de passe interrompu repris, publication
entière ou rien — trois relectures, corrigées) a passé son smoke dans
les quatre formes (66c9e41), étiquetée le 13/10 — première publication
par la tâche finale `publish`.
Les versions 0.398 à 0.420 n'ont jamais été étiquetées : le lanceur est
resté sur la 0.397.0 jusqu'au 13/10 ; désormais chaque version dont le
smoke passe s'étiquette aussitôt ; les étiquettes intermédiaires n'ont jamais été
posées, et **la 0.395.0 ne se publie pas** (elle lisait mal les doses de
charge et les titrations, corrigé dans la 0.396.0).

**Ce qui a été fait le 11/10.** Relectures par domaines, suite :
tables cliniques (foie, grossesse, cytochromes : associations prises
par la ligne d'un de leurs composants, aiguilles sans espace), console
(une récursion arrêtait l'application), biologie (bornes dessinées sur
une autre échelle que la courbe), journal des postes (une correction
illisible masquait un enregistrement ; un enregistrement demandé n'était
jamais envoyé), téléphone (arrivée interrompue sans retour possible ;
conversations entre officines), locations, contenu de référence semé
hors du poste de référence, réécritures du foie au mauvais stade, valeurs
PK, groupes de collègues, feuilles remises, codex. Et une relecture de
mes propres corrections, deux fois : chaque régression trouvée est
corrigée et a son test. Le détail est au CHANGELOG (0.395.0, 0.396.0).

**Après la publication de la 0.397.0 (11/10, après-midi).** Poussé sur
`main` au fil de l'eau (consigne : pousser à chaque lot), étiquette à la
prochaine version dont le smoke passe. Relectures : réseau (un
enregistrement trop gros bloquait toute publication et faisait resceller
le reste à chaque synchronisation ; une valeur revenue ne partait pas),
dose au poids (« dose maximale » lue comme une dose), revue d'ordonnance
(associations fixes seules, fer et bisphosphonate injectables,
« antiprogestatif »), cascades (naloxone des associations), sites des
formes locales, impression (étiquettes, rendez-vous, semaine, bulletins,
fichiers temporaires), carte Vitale (plusieurs fiches sous un NIR),
codes-barres, journal d'accès. Le détail est au CHANGELOG (Unreleased).
Puis (même jour, 12/10) : config.toml illisible écrasé par les défauts,
planning (fin illisible ou avant le début, pause, heures « 12:3 »),
dates de naissance au futur, unités PK, couverture en UI ou gouttes,
insuline humaine, COVID hors saison, compare-and-set (pause d'un poste,
conciliation, acte de vaccination, retour d'une case de table), ADELI,
lanceur, relais.

Laissé, relevé par les relectures (à trancher ou à vérifier) :
- conciliation : une association écrite **avec des espaces seulement**
  (« amlodipine valsartan », « amoxicilline acide clavulanique ») se
  rapproche encore de la fiche d'une de ses molécules par préfixe. Trois
  tentatives par listes de mots ont chacune cassé d'autres fiches (sel
  lu comme seconde molécule, association prise pour le produit seul) et
  sont retirées ; seule l'association écrite (« + », « / ») est gardée.
  Une solution sûre demande sans doute de chercher d'abord une fiche
  d'association qui porte les deux noms ;
- suppression d'un dossier cité au registre, reçue d'un autre poste :
  seule l'identité est gardée ; ses actes, sa biologie et ses notes
  partent avant elle, dans des envois antérieurs (la suppression se
  découpe au changement de flux et à 40 Ko) — les garder demande que
  `delete_patient` émette la suppression du dossier d'abord, ou que le
  receveur regarde plus loin dans les envois de l'auteur ;
- dossier d'échange : un fichier `.bpmposte` se lit enregistrement par
  enregistrement, sans borne de taille (fait le 13/10) ; une réécriture périmée ne
  s'accepte plus qu'en la retapant (un geste « garder le texte livré »
  par phrase reste à faire) ;
- rein : un résultat « <15 » se lit au-dessous de 15, et une borne qui
  laisse un palier ouvert donne « à vérifier » (fait le 13/10,
  `Reading::bound`, `renal::read_bounded`), et les règles de `biology`
  disent « à vérifier » quand une borne chevauche leur seuil (fait le
  13/10), comme la lecture d'intervalle : une borne qui couvre deux
  niveaux se lit « à interpréter », en disant le pire qu'elle permet
  (`biology::reading_level`) ;
  une vingtaine de lignes « Entre X et Y » ont encore `below: Y` (Y
  exclu) — à relire contre chaque RCP (les trois anticoagulant et
  cotrimoxazole sont faits) ; metformine « Entre 45 et 60 » est 45–59
  au RCP ; un DFG ancien n'est pas signalé comme tel au-delà de sa date
  affichée ; le plafond d'une note de posologie (Alfatil « Sans dépasser
  1 g par jour » dans la colonne note) n'atteint pas « Dose par kilo » ;
- paquet et copie : la copie manuelle, le paquet et les copies du jour
  des pièces et du registre passent par étapes (`stepwise_copy`, fait le
  13/10) ; restent sous `VACUUM INTO` les instantanés de la maintenance
  (sans mot de passe) ; l'export tourne encore dans l'image — à passer à
  un fil (un paquet porte depuis le 13/10 la taille et l'empreinte de
  chaque fichier, et un paquet coupé se refuse) ;
- **changement de mot de passe interrompu** : fait le 13/10 — un témoin
  chiffré par le nouveau mot de passe porte l'ancien (`rekey_witness`),
  et l'ouverture avec le nouveau finit le changement (`finish_rekey`) ;
  l'ancien ne défait jamais rien. Un verrou à part (`<base>_mdp.verrou`,
  fichier vide qui reste) est tenu pendant le changement et la reprise —
  au mieux sur un partage réseau, où un verrou peut ne valoir que
  localement. Les autres postes sur la même base reçoivent « file is not a
  database » sans explication ; mots de passe non effacés de la mémoire
  (`zeroize`) ; pas d'action « créer une nouvelle base ici » maintenant
  qu'un chemin écrit et absent est refusé ;
- postes : deux postes invités en même temps par deux postes différents
  reçoivent le même numéro, donc les mêmes blocs d'identifiants (ne
  laisser inviter que la référence ?) ; deux postes qui se désignent
  référence avant de se parler numérotent tous deux (une invitation se
  ferme dans la seconde quand on l'annule ou qu'on verrouille, et « × »
  et « Fonder un autre groupe » se font en deux clics : faits le 13/10) ;
- carnet : deux postes peuvent ouvrir le même flacon dans les deux
  secondes (le lot proposé, le lot périmé ou d'un autre vaccin, et le
  lot reçu déjà périmé : faits le 13/10) ;
- impression d'un entretien : fait le 13/10 (les phrases de `pdf.rs` dans
  `strings.fr.toml`, « — » pour une naissance inconnue et sous une liste
  vide) ;
- le document d'équipe vit dans la base chiffrée (réglage `team_doc`,
  compare-and-set ; l'ancien `notes_equipe.md` repris puis vidé — fait le
  13/10). Reste : deux postes à bases séparées qui les écrivent en même
  temps lèvent une question à trancher (une fusion à la synchronisation a
  été essayée puis retirée : chaque poste fondait dans son ordre, sans
  renvoyer le résultat, et les deux divergeaient en silence — il faudrait
  une fusion déterministe et renvoyée) ; la fusion par lignes d'une même
  base ramène une ligne que l'autre avait rayée ; au-delà de ~16 Ko, les
  notes ne voyagent plus (le volet le dit) ;
- une transmission écrite dans les deux secondes après minuit se range au
  jour nouveau pendant que la page montre encore la veille ;
- tables de référence : les tables corrigées se relisent contre
  `table_rev` (fait le 13/10 ; `all_table_cells` n'est lu qu'au clic et
  dans la recherche mémorisée) ; survol de la carte : `GraphPairKey`
  suit `file_rev` depuis le 13/10 ; la note de pied de la carte est mesurée avec 99 et
  sans `graph_hidden_ring` ; infobulles des arêtes pendant un
  glissement ;
- suppression d'un dossier entre postes : une ligne (biologie, dose,
  note…) écrite ailleurs avant que la suppression arrive est rangée
  quand même, orpheline ; une modification du patient arrivée après la
  suppression garde la ligne entière dans une question « ABSENT » ; et
  le journal des postes garde tout ce qui a voyagé — supprimer n'est
  pas effacer (à dire pour une demande d'effacement RGPD) ; les
  vaccinations et actes facturés partent avec le dossier (obligation de
  conservation à vérifier) ;
- réglages par poste qui devraient être de l'officine : facturation,
  règles (dont `enforcement = Block`), mentions et forfaits de location
  vivent dans `config.toml` — deux postes peuvent facturer ou bloquer un
  même patient autrement ; seule `[pharmacy]` est passée à la base ;
- options ouvertes au verrouillage : mises de côté et rendues au
  déverrouillage (fait le 13/10) ;
- réécritures orphelines : une clé dérivée d'un libellé (âge, grossesse,
  écrasement, rein, foie, revue, surveillance) qui change de libellé
  laisse la réécriture en base, invisible et non comptée ; à montrer
  dans « Textes » (« réécriture sans phrase ») et au rapport de
  maintenance, avec une table de renommage ; « Rétablir » un document
  n'est pas comparé à ce que l'écran montrait ;
- fil du dossier : seules les rangées visibles se dessinent, et le
  résumé se relit contre `fil_rev` (fait le 13/10) ;
- **mises à jour non signées** : le lanceur installe et exécute ce que
  GitHub Releases sert, vérifié par TLS, la taille annoncée par la même
  API et quatre octets magiques — aucune signature. Un accès en
  écriture aux publications (jeton fuité, action épinglée sur une
  étiquette mobile — `softprops/action-gh-release@v2`,
  `Swatinem/rust-cache@v2` — ou cache empoisonné) atteint chaque
  officine. À faire, avec une clé hors ligne à créer par Youssef :
  signer les binaires (minisign/ed25519), embarquer la clé publique
  dans le lanceur, refuser sans signature (les actions sont épinglées
  par empreinte et la publication se fait sans cache depuis le 13/10) ;
- `release.yml` : les constructions passent leurs fichiers à une tâche
  finale `publish`, qui exige les six binaires des postes avant de
  publier (fait le 13/10, à voir tourner à la 0.430.0 : la 0.427.0 était
  « latest » vingt-six minutes avec l'APK seul) ; reste `prerelease`
  pour une étiquette à suffixe ;
- postes : retirer un poste ne change pas la clé — un poste retiré (ou
  un téléphone retiré, avec sa part) qui se tire une identité neuve
  écrit encore, et peut inscrire sa propre ligne de `sync_posts` ; la
  vraie révocation demande de tourner la clé au retrait, et l'amorce
  d'un nouveau poste par celui qui l'invite plutôt que par lui-même ;
- `bpm-sync` : un `Give` est accepté sans `Want` en attente, et des
  parents inventés près de `0x00…` passent devant les vraies têtes dans
  `need` — un membre peut étirer ou faire échouer une séance ;
- grossesse : le texte « molécule absente
  de la table » de `gravidity.rs` n'a pas d'adresse de réécriture, et
  `rhythm_text` (« tous les mois ») est écrit en dur ;
- ordonnance après TROD : la nitrofurantoïne en 3e intention de la
  cystite simple, et l'amoxicilline de l'enfant « maximum 50 mg/kg/j »
  sans plafond adulte — à relire contre le texte du protocole ;
- textes imprimés hors `strings.fr.toml` : en-têtes de la grille de
  prises (« Médicament », « Indication », moments, repas), « Conseils »
  et « Relu en » dans `pdf.rs`, l'intitulé du protocole sur
  l'ordonnance, les manques du renouvellement ;
- conciliation : « 1 000 mg matin et soir » contre « 1 g matin et soir »
  se lit « dose changée » (sens sûr, mais bruyant) ;
- location au mois : une location commencée le 29, le 30 ou le 31
  janvier atteint son mois le 28 février (`add_months` borne au dernier
  jour) — du 31/01 au 28/02, deux forfaits ; règle à trancher ;
- fiches conseils (protections, etc.) : les titres de fiche et de
  section s'impriment sans passer par `content.rs` — seules les lignes
  se réécrivent ;
- tableau de bord : la liste d'appels (biologie) se recalcule après
  chaque écriture d'un autre poste tant que la vue est ouverte — juste,
  mais une passe sur la base ;
- l'oxomémazine (Toplexil) n'est pas dans la table du sujet âgé — à
  vérifier contre la liste de Laroche avant de l'y mettre ;
- dTcaP : un rappel fait à 64 ans rend celui de 65 ans « dû » un an
  plus tard ; la règle de transition du calendrier est à vérifier ;
- insuline lispro biphasique : Humalog Mix 25 et Mix 50 se lisent sur la
  même courbe ;
- planning : une nuit à cheval sur un changement d'heure se compte en
  heures d'horloge (une heure de plus ou de moins) ;
- `delete_patient` ne compare que l'identité : un acte ou une dose
  ajoutés ailleurs entre-temps partent avec le dossier ;
  `delete_protocol_node` lit l'arbre hors transaction ; plusieurs
  suppressions ne comparent qu'un champ (vaccination, posologie, poste,
  fiche) ; `reset_table` est sans comparaison ; `rename_protocol`,
  `rename_checklist`, `update_protocol_node` écrivent un champ qu'ils ne
  comparent pas ;
- lanceur sous Windows : lancé pendant que l'application tourne, il
  télécharge puis échoue au remplacement, et lance une seconde instance ;
- relais : un membre du réseau peut occuper les quatre places d'attente
  d'un groupe (déni de service, pas de fuite).
- la fenêtre d'audit (`bpm-audit`) ouvre la base par `Db::open`, qui
  migre et peut créer les fichiers voisins : il lui faudrait une
  ouverture en lecture seule ;
- le dossier `backups/` est créé avec les droits par défaut (0755) : le
  restreindre à 0700 fermerait un dossier partagé aux postes d'autres
  comptes, donc laissé ;
- la 0.398.0 a passé le smoke et la CI ; l'étiquette n'a pas pu être
  posée d'ici (refusée par les permissions) : `git tag v0.398.0 277eaa3
  && git push origin v0.398.0` ;
- registre : deux lignes du même jour pour une même délivrance en deux
  lots se lisent comme deux délivrances (« rapprochement » à 0 jour) ;
  c'est voulu par un test, mais une délivrance en deux lots y tombe —
  à trancher ; l'ordonnancier imprimé donne le libellé actuel d'un produit
  renommé ; dans un jour, l'ordre du registre suit l'heure d'écriture de
  chaque poste (`created_at`) : il se fie aux horloges des postes, et un
  poste en retard de quelques minutes place sa ligne avant une ligne
  écrite plus tôt ailleurs ;
- actes : un acte jamais tenu (« Identifié » puis abandonné, rendez-vous
  manqué laissé « Planifié ») prend un rang et entame le quota ; il n'y a
  pas d'état « abandonné » — à trancher (le rang ne compterait que les
  actes tenus, ou un état de plus) ;
- versions entre officines (relevé le 12/10, à reprendre ensemble) :
  l'historique se range maintenant à l'heure d'écriture (`card_edits.at`,
  12/10) ; un poste autre que la référence reçoit sans versionner d'abord
  ce qu'il a changé, et une modification d'ici peut alors « répondre »
  à une version en attente sans clic ; renommer une fiche, une
  préparation ou un protocole les duplique chez les autres (pas
  d'origine figée, contrairement aux lignes TROD et aux vaccins) ; une
  version reçue recrée vide une fiche, une préparation ou un protocole
  supprimé ici ; un retour d'ici à la valeur d'avant une version reçue
  appliquée ne se versionne pas ; une valeur TROD que l'écriture
  normalise (espaces, lignes vides) reste à arbitrer pour toujours ;
- agenda : une entrée ne passe pas minuit (refusée), une série ne peut
  pas s'arrêter à une date (seule la suppression, qui emporte le passé) ;
  au passage de minuit du dimanche, la semaine affichée et l'impression
  du jour peuvent se désaccorder ;
- campagne : un poste d'une version plus ancienne qui enregistre une
  campagne efface les réglages de sujet qu'il ne connaît pas ; aucun
  moyen d'écarter d'une campagne à l'autre un patient décédé ou parti ;
  une dose saisie sans code compte au bilan sous son libellé mais laisse
  le patient dans la liste de rappel ; l'éditeur accepte doses « 0 »
  (lu 1), âge au-delà de 120, flacon « 0 » sans le dire ;
- biologie (contenu clinique, à relire avant de toucher) : la règle du
  TP bas sous paracétamol sonne l'alerte chez tout patient sous AVK, pour
  qui un TP bas est attendu (il faudrait une exclusion) ; trois paires de
  règles se recouvrent à des seuils différents (CRP > 100 et > 50, Hb
  < 10 et < 11, PNE > 1,5 et > 0,5) et répondent deux fois — le test
  n'écarte que les seuils égaux ; « au-dessus » inclut la borne haute de
  l'intervalle (K 5,0 « normal » au tableau, « élevée » à la lecture) ;
  le sexe du patient n'est pas lu pour les intervalles (Hb, créatinine,
  hématocrite) ; les signaux de la carte (`companion_signals`) et la
  veille du tableau de bord donnent la phrase livrée, pas la réécrite ;
- feuilles de conseil : le titre de la feuille et les titres de section
  s'impriment sans être réécrivables (`conseils::phrases` ne les liste
  pas) ; le carnet de glycémie n'a pas de ligne pour l'hyperglycémie
  (un seuil à prendre dans la source SFD avant de l'écrire) ;
- **tables de référence : les cases réécrites sont rangées par numéro de
  ligne** (`table_cells (table_key,row,col)`). Une ligne livrée insérée
  au milieu décale toutes les réécritures suivantes : la 0.371.0 a inséré
  « Anticorps contre le VRS (nourrisson) » en 8e ligne de « Vaccins », et
  une réécriture plus ancienne de la ligne HPV s'affiche depuis sur la
  ligne VRS, celle de ROR sur la suivante… Il faudrait les ranger par
  le texte de la première colonne, avec le texte livré vu (comme
  `content_overrides`), et relire les réécritures de « Vaccins » des
  officines concernées. **Fait le 12/10 pour l'avenir** (`row_label`,
  refile avant écriture) ; les réécritures d'avant, sans étiquette,
  gardent leur numéro, et celles de « Vaccins » faites avant la 0.371.0
  restent décalées d'une ligne : à relire chez les officines qui en ont ;
- l'insuline NPH et NovoMix durent 24 h sur la courbe et 12 à 16 h (10 à
  16 h) dans la table « Insulines » ;
- ruptures : aucun écran n'écrit de retrait (une substitution cliquée
  par erreur compte pour toujours) ; une rupture « Diprosone » ne se voit
  pas sur la fiche « bétaméthasone » (la DCI voyage mais n'est pas lue) ;
- **messagerie entre officines sur plusieurs postes** : deux postes qui
  synchronisent le réseau avant de s'être répliqués rangent chacun le même
  message reçu (sous deux numéros), et la réplication porte les deux : le
  message s'affiche deux fois, une conversation nouvelle se dédouble, et
  le second poste republie ce que le premier a écrit (`from_device` vide
  voyage). Rien n'impose l'unicité de `uid` dans `conversations`,
  `messages`, `message_files`. À trancher : réseau réservé à un poste
  (comme `seeds_here`), ou dédoublonnage par `uid` à l'application —
  l'index unique ne peut pas s'ajouter sans nettoyer les doublons déjà
  rangés ;
- codex : un point devant trois chiffres reste lu comme séparateur de
  milliers même dans une formule qui écrit ailleurs « 12.5 » (« 2.500 g »
  lu 2 500) ; « À peser » laisse un pourcentage tel quel (pas de masse) et
  « 5 % soit 5 g » ne recalcule pas sa masse ; la colonne « Titre » donne
  la part d'une solution diluée, pas la concentration de son principe ;
- téléphone : la suppression d'un rendez-vous de l'agenda ne compare que
  le titre ; la base peut s'ouvrir deux fois (deux déverrouillages) et
  n'est jamais refermée en quittant par Retour. Le code Kotlin modifié le
  12/10 (FLAG_SECURE, verrou, brouillon, fiche, phrase) n'a pas pu être
  compilé ici (pas de SDK Android) : c'est la CI qui le compile.

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
le journal des accès — il peut aussi, plus simplement, écrire un message
dans une conversation entre officines existante, que le bureau publie
sous le nom de l'officine (`admissible` ne regarde que le flux, et
`outgoing_net_messages` ne regarde pas quel poste a écrit) ; un
téléphone modifié peut encore écrire `left_on` ou `reach` sur les lignes
des postes de bureau telles que les autres téléphones les voient (les
bureaux le refusent, pas les téléphones) ; un téléphone se déclare lui-même `kind=''` (poste
de bureau) à son arrivée ; un téléphone retiré garde sa clé, faute de
renouvellement des clés. Les trois demandent que le poste qui invite
écrive lui-même ce qu'il a invité.

**À trancher aussi — l'ordonnancier et l'article R. 5132-10.** Le
registre des stupéfiants (R. 5132-36) tient : ajout seul, annulations
barrées, solde, écarts motivés. L'ordonnancier imprimé, lui, ne porte
pas ce que l'article demande :
- **le nom et l'adresse du patient** : la ligne porte « dossier N »
  par choix (le fichier du registre se remet seul à une inspection) et
  aucune impression ne résout le numéro en nom et adresse ; il faut au
  moins une impression « pour l'inspection » qui le fasse, et décider
  si l'identité du jour de la délivrance doit être figée (le dossier
  reste modifiable après coup) ;
- **le prescripteur** est un texte libre, sans adresse, et n'est pas
  exigé ; **le porteur** (quand ce n'est pas le patient) n'a pas de
  colonne ;
- **listes I et II** : l'« Ordonnancier » ne tient que les stupéfiants
  et assimilés, alors que la recherche promet « liste I, liste II » —
  dire qu'un autre logiciel tient celui-là, ou l'ajouter ;
- les numéros d'ordonnancier viennent désormais du logiciel de
  l'officine, tapés sur chaque délivrance (12/10) : deux postes qui
  tapent le même avant de se synchroniser gardent les deux lignes, et
  l'ordonnancier dit le numéro porté deux fois ; le choix de la bonne
  ligne reste à faire à la main (une annulation de l'autre).
Corrigé : une remise à zéro garde les dossiers que le registre cite.

**Application Android (relevé, Kotlin non construit ici).** Joindre un
fichier échoue sous Android 8 à 12 (`readNBytes` n'existe qu'à partir
de l'API 33) ; une clé du Keystore perdue (verrouillage retiré) laisse
le téléphone bloqué à l'écran de déverrouillage, la suppression du
coffre n'étant offerte qu'une fois la base ouverte ; la reconnaissance
faciale « faible » réussit à l'invite puis échoue à la clé (Android
9-10) ; une pièce ouverte reste déchiffrée dans le cache après
verrouillage ou effacement ; deux appels au pont hors `runCatching`
peuvent planter si la base se ferme pendant. Corrigés côté pont :
pas de suppression d'une série d'agenda d'un appui, initiales en
majuscules, conversations privées filtrées.

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

**Comparaisons restantes (corrigé le 12/10).** Les champs d'un acte
comparent à la ligne qu'ils montraient (`itv_seen`) ; un champ non touché
suit la relecture. Reste : une DDR en cours de frappe compare encore à la
valeur relue, et la date d'ordonnance tapée aussi (`script_date_edit`).

**Ordonnances TROD (relevé).** Grossesse inconnue lue comme « pas
enceinte » (aucune ligne ne le rappelle) ; aucune barrière d'allergie
(l'amoxicilline est proposée quoi que dise le dossier).

**Codex (corrigé le 11/10).** « 1.000 mL » se lit mille dans une formule,
sa quantité préparée et la quantité voulue ; ailleurs (biologie,
registre), un point reste décimal.

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
ensuite. Le téléphone refuse depuis le 11/10 toute conversation qui
n'est pas de l'équipe (lire, fichier, lu, écrire) ; il ne vérifie pas
qu'une conversation privée de l'équipe le concerne (la fonction ne
reçoit pas les initiales).

**Feuilles remises (corrigé le 11/10).** Le plan de prise, la fiche et
les étiquettes impriment la posologie que la base garde, et non le champ
en cours de saisie.

**Petits restes.** Bulletins : un champ trop long pour sa case, même à
5 pt, est dit à l'impression (fait le 13/10). Console : les tables comptent les lignes
qu'elles rendent, et un script s'arrête à deux millions (fait le 13/10).

**Relecture de mes propres corrections (relevé, laissé).**
- *Ruptures* : un retrait ne vaut que de l'officine qui a écrit
  l'événement, comparée par son **nom affiché** (`source`). Renommée ici
  entre les deux, l'officine ne lève plus sa propre rupture, qui reste
  « en cours » jusqu'à l'horizon de 90 jours. Le bon repère est l'appareil
  signataire, ce qui demande de le garder sur l'événement (colonne et
  migration).
- *Grippe* : la saison est fixée de septembre à février. Une campagne dont
  la fin est repoussée en mars ne liste plus personne après février.
- *Table rénale* : les paliers de la cefpodoxime (39 → 40) et de Tarka
  (30 → 31) ont changé d'adresse ; une réécriture faite par l'officine
  sous l'ancienne adresse revient au texte livré (voulu, et dit dans le
  commit).
- *Pièces* : sans aucune fiche en base, les octets restés dans
  `_scans.db` ne sont jamais compactés — le choix prudent face à une base
  remise en place.
- *Contenu de référence* : « Synchroniser le contenu de référence »
  (À propos) passe par `insert_missing_drugs` et `seed_posologies`, qui
  remettent une fiche livrée ou ses posologies que l'équipe a
  supprimées (aucune marque « semé », contrairement aux protocoles et
  aux préparations). Voulu pour « Compléter les médicaments de départ »
  (un test le dit), probablement pas pour « Synchroniser », dont l'aide
  promet « vos textes sont conservés ». À trancher : une marque par
  fiche semée, et « Synchroniser » qui la respecte.
- *Textes réécrits* : « Revenir au texte livré » sur tout un document
  (`reset_content`) efface aussi une réécriture qu'un autre poste a faite
  après l'ouverture de l'écran, sans avis.
- *Annuaire des prescripteurs* : choisir une ligne écrit « Dr Nom »
  seulement ; deux homonymes deviennent le même prescripteur, et la
  vigilance « plusieurs prescripteurs ? » du registre compare ces
  chaînes. Garder le RPPS avec la ligne (colonne et migration) et
  comparer sur lui.
- *Ordonnances protocolisées* : l'amoxicilline de l'enfant (50 mg/kg/j)
  s'imprime sans dose maximale par jour — à vérifier contre le texte du
  protocole (non consultable ici), rien n'a été inventé. Pour la
  grossesse inconnue, voir « Ordonnances TROD » plus haut ; la feuille du
  TROD cystite liste « grossesse en cours ou possible » parmi les signes
  qui excluent.
- *Posologies* : un jour nommé n'importe où dans la ligne (« contrôle INR
  jeudi ») la sort de la grille ; dans le doute c'est le sens qui ne
  coûte qu'une question.

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
