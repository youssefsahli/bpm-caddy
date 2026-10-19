# Synchronisation pair-à-pair — état de la construction

`ROADMAP.md` portait depuis le début une ligne restée seule :

> Sur l'onglet « officine » : on devrait pouvoir régler une url, puis
> travailler l'UI/API pour avoir de la synchronisation (données
> chiffrées).

C'est le seul point de la feuille de route qui ouvre un chemin réseau
dans une application dont toute la posture tient en une phrase : tout
est local, la base est chiffrée, et la seule requête sortante est la
vérification de version, sur un bouton. Ce document dit ce qui a été
construit — le crate `sync/`, `bpm-sync`, entièrement testé et branché
sur rien —, ce qu'il garantit, ce qu'il ne garantit **pas**, et ce qu'il
reste à faire pour qu'une officine s'en serve.

Il s'adresse à qui reprend le sujet, y compris à moi-même dans six mois.

> **Depuis 0.273.0, un premier usage est branché : le réseau
> d'officines** (`src/network.rs`). Il ne transporte qu'un flux,
> `Stream::Reseau` — les ruptures et ce qu'on a donné à la place, sans
> aucun patient —, sous un trousseau **propre au réseau** qui n'ouvre rien
> d'autre. Les décisions du § 8 ont été prises pour lui par l'officine :
> entre officines d'un groupement ; sur un bouton et à la fermeture ;
> personne n'écoute en permanence (une porte s'ouvre le temps d'une
> invitation) — **décision revue en 0.321.0, sur demande de l'officine :
> une porte tenue ouverte aux seules officines appairées, voir plus
> bas** ; et, en plus des adresses, un **dossier d'échange** où
> chaque officine dépose ses enregistrements scellés. La fonction `sync`
> est donc allumée par défaut ; `--no-default-features --features desktop`
> rend toujours un binaire sans aucun code réseau. Les autres flux (dossiers, registre,
> caisse…) restent non branchés, et le § 7 vaut toujours pour eux.

---

## 1. Règle d'entrée : optionnel signifie absent

`bpm-sync` est un crate séparé, dépendance **optionnelle** de
l'application, derrière `--features sync`, éteint par défaut. Ce n'est
pas une commodité de compilation : un binaire construit sans ce drapeau
ne contient ni socket d'écoute, ni machine à états, ni rien à attaquer.
Une officine qui ne veut pas de réseau n'obtient pas « une option
décochée », elle obtient un logiciel où le code n'existe pas.

Le crate ne dépend de rien de l'application : il ne lit aucune base, ne
dessine aucun pixel, et ne sait pas ce qu'est un patient. Il reçoit des
octets et rend des octets. C'est ce qui permet de faire dialoguer deux
postes entiers — appairage, échange de clé, divergence comprise — dans
un seul `#[test]`, sans ouvrir de socket : la seule façon qu'un
protocole soit vraiment regardé.

Il est en revanche **membre de l'atelier**, donc l'intégration continue
le compile et le teste à chaque passe. Optionnel ne veut pas dire non
surveillé.

---

## 2. Le modèle de menace

### Périmètre protégé

Le journal clinique d'une officine : ce qui a été délivré, à qui, par
qui, et les corrections. Contre trois choses.

* **Qui transporte.** Le commutateur de l'officine, le routeur, la
  liaison entre deux sites, et — le jour où il en existe un — un relais.
  Ils portent des octets qu'ils ne peuvent pas lire, et ils ne peuvent
  pas en ajouter.
* **Qui se met au milieu.** Une poignée de main chiffrée prouve qu'on
  parle à quelqu'un qui détient *une* clé ; elle ne dit pas que c'est
  l'autre poste de l'officine. Seuls deux humains qui se lisent un code
  peuvent le dire, et c'est exactement ce que l'appairage demande.
* **Qui rejoue.** Une conversation enregistrée ne se rejoue pas : la
  signature d'identité porte sur l'empreinte de *cette* poignée de main.

### Hors périmètre

* **Pas de confidentialité persistante (forward secrecy) au repos.** Le
  trousseau est de longue durée, parce qu'un poste qui rejoint
  l'officine en mars doit pouvoir lire ce qui a été écrit en janvier :
  un pharmacien qui ouvre un dossier veut le dossier, pas la partie du
  dossier postérieure à son portable. Qui obtient le trousseau **et**
  les enregistrements scellés les lit tous. La défense est celle que
  l'application a déjà : tout cela vit dans SQLCipher, sur un disque
  qui appartient à quelqu'un, derrière le mot de passe de l'officine.
* **Pas de défense contre un poste dont le disque et le mot de passe
  sont entre d'autres mains.** C'est un transport et une histoire, pas
  une deuxième serrure sur la porte.
* **Une image de trafic reste visible.** L'en-tête d'un enregistrement
  est en clair — il le faut, pour qu'un pair le place dans le graphe
  sans pouvoir le lire. Qui détient le journal sans le trousseau
  apprend donc que tel poste a écrit onze lignes de registre un mardi.
  Il n'en apprend pas un mot.
* **Révoquer n'est pas reprendre, et révoquer se fait partout.** Sortir
  un poste de la liste d'une machine l'empêche de se connecter *à cette
  machine-là*. Tout ce qu'il a écrit reste attribuable pour toujours —
  c'est signé, et `Fact` porte l'auteur. Mais un poste que les autres
  ont désavoué peut encore donner un enregistrement neuf à un poste qui
  ne l'a *pas* désavoué, et de là il arrive partout : un journal accepte
  ce qu'on lui donne, parce que refuser un auteur ferait diverger pour
  toujours deux journaux sur une liste qu'aucun des deux ne voit.
  Révoquer veut donc dire révoquer sur chaque poste. Et cela ne reprend
  pas ce que la machine détient déjà : pour le passé, il faut re-clé
  l'officine, exactement comme la seule réponse à une base volée est
  d'en changer le mot de passe. **La révocation est affaire d'avenir ;
  la re-clé est affaire de passé.**

### Re-clé : coût

Une officine ne jette pas quatre ans de registre parce qu'un poste a été
volé. Re-sceller tous les enregistrements sous la nouvelle clé les
**renommerait** tous — le nom est un hachage du chiffré — et un journal
renommé est un autre journal : ce n'est donc pas la voie.

La voie est celle-ci, et elle est déjà en place : la nouvelle clé scelle
ce qui s'écrit à partir de maintenant, les anciennes restent dans la
base, et `Journal::read_with` les essaie dans l'ordre, la plus récente
d'abord. Rien ne change sur le fil, aucun enregistrement ne bouge, et il
n'y a **pas de marqueur d'époque** dans l'en-tête à tenir à jour : le
sceau répond tout seul, puisqu'un AEAD sous la mauvaise clé ne s'ouvre
pas. Un enregistrement qu'aucune clé du trousseau n'ouvre reste
**nommé**, jamais sauté.

Ce que la re-clé ne rend pas : ce que le poste volé détient déjà. Elle
coupe la suite, pas le passé — la même chose exactement que changer le
mot de passe d'une base dont quelqu'un a pris une copie.

---

## 3. Les quatre garanties et leurs règles

### « Un pair ne voit jamais de clair »

Les enregistrements sont scellés avant d'être nommés
(XChaCha20-Poly1305, clé dérivée du trousseau par flux). Le nonce est
tiré au hasard à chaque fois : vingt-quatre octets, assez larges pour
qu'aucun poste n'ait à coordonner un compteur avec les autres — ce qui
est la seule disposition qui survive à trois machines qui écrivent
pendant que l'une d'elles est débranchée.

*Ce qui le tient :* `a_relay_cannot_read_what_it_carries` ramasse **tous
les octets** qui ont traversé une conversation complète et y cherche les
phrases écrites, le trousseau, et les graines des deux postes. Mesuré
sur les octets, pas argumenté depuis le dessin.

### « Un enregistrement ne se réécrit pas »

Son nom est le hachage de son contenu, et les suivants le nomment comme
parent : changer un octet le renomme et casse tout ce qui a été écrit
après. Une correction est un **autre** enregistrement qui dit lequel il
corrige ; le fautif reste écrit, et sur le papier il reste barré. C'est
la règle que l'ordonnancier de stupéfiants suit déjà (R. 5132-36),
généralisée : un journal qu'on peut éditer ne prouve rien, et un qu'on
peut éditer *à distance* prouve moins que rien.

*Ce qui le tient :* `a_record_is_named_by_its_content`,
`a_correction_never_removes_what_it_corrects`, et
`a_journal_can_only_ever_be_added_to`, qui lit le texte du module et
refuse le verbe qui enlèverait un enregistrement — comme
`the_register_can_only_ever_be_written_to` le fait dans `db.rs`.

### « L'horloge de personne ne décide »

L'ordre vient d'un compteur de Lamport et du nom de l'auteur. Un poste
dont la date est fausse écrit une ligne au mauvais endroit d'un
*calendrier* ; il ne peut pas réordonner le journal, ni faire passer une
ligne ancienne sous une récente. L'ordre est **total** (rang, puis
auteur, puis nom), sans quoi deux postes détenant les mêmes
enregistrements les afficheraient dans deux ordres — et deux écrans qui
montrent un registre différemment, personne ne peut agir dessus.

### « Un conflit se montre, il ne se tranche pas »

Deux postes qui corrigent le même enregistrement sans s'être vus : les
deux corrections restent, **et chacune nomme l'autre**, dans la donnée
et non à côté. C'est le refus que les avertissements de
compare-and-set font déjà au comptoir, pour la même raison : choisir en
silence entre deux affirmations cliniques, c'est se tromper une fois sur
deux avec l'assurance d'une réponse.

Et le corollaire, qui est la règle que la maison écrit partout :
**le silence ne vaut pas autorisation.** Un enregistrement que le
trousseau en main n'ouvre pas est **nommé** (`Reading::unopened`), pas
sauté : une liste discrètement amputée d'une ligne se lit exactement
comme une liste complète.

---

## 4. La forme du crate

| module | ce qu'il répond |
| --- | --- |
| `enc.rs` | l'encodage canonique — écrit à la main, parce que le *nom* d'un enregistrement est un hachage de ces octets-là |
| `keys.rs` | `Device` (cette machine, qui signe) et `Trousseau` (l'officine, qui ouvre) ; les empreintes lues à voix haute |
| `seal.rs` | un enregistrement : scellé, nommé par son contenu, signé |
| `journal.rs` | le graphe, la fusion, la lecture d'un flux, et ce qu'un pair doit recevoir |
| `wire.rs` | six trames, et rien d'autre |
| `session.rs` | le protocole comme machine à états qui ne touche à rien |
| `meter.rs` | les compteurs — un volet, pas une balise |
| `link.rs` | le seul endroit où des octets bougent : la porte, le lien, et de quoi frapper |

Tout sauf `link.rs` est pur. L'aléa est **passé** (`Entropy`), comme la
date est passée partout ailleurs dans cette application. La seule
exception est la clé éphémère de la poignée de main, que `snow` tire du
système : une poignée de main dont un test pourrait fournir l'aléa est
une poignée de main à laquelle il ne faut pas se fier, et faire semblant
du contraire par symétrie serait le mauvais échange.

### L'API, en entier

```rust
let device    = Device::generate(&mut OsEntropy);    // ce poste
let trousseau = Trousseau::generate(&mut OsEntropy); // cette officine
let mut journal = Journal::new();

// Écrire un fait, lire un flux. Ce qui entre, ce sont des octets : ce
// crate ne sait pas ce qu'est une ligne de registre, et ne doit pas.
journal.write(&device, &trousseau, Stream::Registre, octets, None, &mut e)?;
let lecture = journal.read(&trousseau, Stream::Registre);

// Parler à un poste. `drive` porte la boucle, parce qu'il n'y a qu'une
// façon correcte de l'écrire : `confirm` reçoit le code, l'écran le
// montre, et répondre « non » arrête la conversation au lieu de la
// continuer en silence.
let mut session = Session::new(&device, Some(&trousseau), Intent::Sync, true, &connus)?;
let mut link = link::dial("192.168.1.14:7742", patience)?;
drive(&mut session, &mut link, &mut journal, &mut meter, &mut |code| {
    montrer(code.groups()) == Reponse::MemeCode
})?;
```

Et de l'autre côté : `let door = link::Door::open("0.0.0.0:7742")?;`
puis `door.accept(patience)?`, avec `Intent::Invite` la première fois.

C'est tout. `Session::step` / `deliver` restent publics pour qui veut
conduire la conversation autrement — sur autre chose qu'un socket, ou
au rythme d'une interface — mais personne n'est obligé de les toucher.

---

## 5. Le protocole

**La poignée de main n'est pas inventée ici.** C'est Noise XX
(`Noise_XX_25519_ChaChaPoly_BLAKE2s`), via `snow`. Un protocole écrit
pour une seule application est un protocole que personne n'a attaqué, et
celui-ci porte des données de santé.

Ce que XX donne : un canal chiffré où chaque bout a prouvé détenir *une*
clé statique. Ce qu'il ne peut pas donner : une raison de croire que
cette clé est celle de l'autre poste de l'officine plutôt que de ce qui
s'est mis au milieu. Deux choses comblent l'écart, et aucune n'est
facultative.

1. **L'identité est liée au canal.** Juste après, chaque poste envoie un
   `Hello` portant son `DeviceId` et une signature Ed25519 *sur
   l'empreinte de la poignée de main*. Un enregistrement d'hier ne se
   rejoue sur rien.
2. **Un inconnu est admis par un humain, une fois.** Les deux opérateurs
   voient les mêmes cinq groupes de quatre caractères, dérivés de cette
   même empreinte, et se les lisent. Égaux : personne au milieu.
   Différents : quelqu'un au milieu, et l'appairage est refusé. On ne
   peut pas sauter l'étape — un poste non déjà connu n'est pas écouté
   tant que `accept()` n'a pas été appelé, et la seule chose qui appelle
   `accept()` est une personne — ou un ticket.
3. **Le code d'invitation remplace la comparaison, sans l'affaiblir.**
   L'officine qui invite tire un `Ticket` (dix octets) pour cette seule
   porte et le montre dans son code d'invitation, transmis par un canal
   qu'elle choisit (téléphone, en personne). Une fois les `Hello`
   échangés, le poste qui rejoint envoie `Proof` : BLAKE3 à clé (dérivée
   du ticket) sur son rôle et **l'empreinte de cette poignée de main** —
   ou `Proof(None)`, sans ticket. L'invitant vérifie, répond par sa
   propre preuve (l'autre rôle), puis seulement envoie la clé. Un
   intermédiaire tient deux poignées de main, donc deux empreintes, et
   n'a de preuve pour aucune sans le ticket. **Une invitation à ticket
   est stricte** : `Proof(None)` y est refusé, sans repli sur la
   comparaison — une porte ouverte qu'un tiers devance ne doit pas
   devenir une comparaison bâclée. Une preuve fausse ferme l'invitation :
   quatre-vingts bits, une tentative. `Sync` ne prend pas de ticket.

**Les officines qu'on n'a pas ajoutées sont comptées, jamais gardées.**
Le dossier d'échange montre aussi ce que déposent des officines qui
détiennent la clé du réseau sans avoir été ajoutées ici (qu'un autre
membre a fait entrer, ou qu'on a retirées — la clé ne change pas). 0.318.0
les gardait au journal ; une revue de sécurité a montré qu'un tel auteur
pouvait alors **effacer** l'enregistrement d'une officine appairée (une
« correction » n'est pas liée à l'auteur de ce qu'elle corrige) ou porter
le rang de Lamport au plafond et brouiller pour de bon l'ordre du réseau,
et qu'un dossier inondé d'identités jetables figeait la vue. Depuis
0.320.0 : le dossier ne les fait pas entrer au journal ; ils sont
seulement **comptés** (32 identités au plus, les écartées non comptées)
et rapportés à
l'écran par la synchronisation ; la carte les propose, avec une alerte
quand le nom qu'elles écrivent est celui d'une officine déjà ajoutée. Les
ajouter (deux clics) relit le dossier, qui garde alors ce qu'elles
déposent ; le nom qu'elles écrivent n'est pas repris comme nom. Une
officine retirée est écartée d'office (`net_ignored`).

Ce qu'une **conversation directe** relaie d'auteurs qu'on n'a pas ajoutés
(les officines d'une officine appairée) reste gardé et relayé, comme
avant — le redemander à chaque échange coûterait tout le trafic. Mais
`absorb` lit à travers une **vue de confiance** (`trusted_view` : cette
officine et les siennes) : une correction venue d'un autre n'y efface
rien. Un tel auteur relayé ne règle pas non plus le rang de Lamport d'ici
(`Journal::trust`, 0.321.0).

**La connexion au lancement** (0.321.0). Le fil automatique, lancé à
chaque ouverture, tient une porte sur `[reseau] port_ecoute` (7745 ;
`ecouter`, oui par défaut) dès que l'officine est d'un réseau — **distinct
de `port`**, celui des invitations, que la porte tenue ouverte aurait
sinon pris. `Session::answer_any` répond
pour **n'importe lequel** de ses réseaux : son `Hello` ne nomme aucune
officine, celui de l'appelant nomme la sienne, qui doit être l'un des
réseaux d'ici *et* connaître l'appelant — sinon refus, avant le moindre
enregistrement ; `drive_any` ouvre alors l'échange sur le journal de ce
réseau. Chaque conversation a son fil, deux au plus, huit secondes par lecture
et **une minute en tout** (`TcpLink::with_deadline` : le délai par lecture
est une option de socket, et qui envoie un octet toutes les sept secondes
le satisfait sans fin) ; les appels sortants ont leur propre compte. L'annonce `BPMOFFICINE2` dit le port d'écoute et la ville
(celle de la configuration, sans code postal) ; une officine **appairée**
qu'on entend est composée à cette adresse (`dial_heard`), une fois toutes
les dix minutes au plus, et l'adresse n'est **gardée qu'une fois que
l'officine attendue y a répondu** — une annonce ne prouve rien, et une
annonce contrefaite ne remplace pas une adresse saisie. La ville annoncée
ne s'affiche que pour une officine qu'on n'a pas ajoutée. Au-delà du réseau local, sans serveur,
rien ne se découvre : restent les adresses saisies et le dossier.

**Entre deux sites** (0.326.0) : une officine joignable de l'extérieur
(routeur, VPN) déclare son adresse (`net_public_address`, fenêtre du
réseau) ; `publish` l'envoie une fois par valeur, **aux seuls membres du
réseau**, chiffrée et signée. `absorb` range la dernière adresse annoncée
par chaque officine appairée à part (`net_announced:<empreinte>`), jamais à
la place de l'adresse saisie ; la synchronisation l'essaie pour une
officine sans adresse ou dont l'adresse ne répond plus, et ne la retient
qu'une fois l'officine attendue jointe (`dial_heard`). Revue de sécurité
(0.328.0) : **une adresse IP et un port, rien d'autre**
(`usable_announced` — ni nom d'hôte, dont la résolution n'a pas de délai,
ni boucle locale, lien local, multidiffusion ou port nul), vérifiée à
l'enregistrement et à la réception ; jamais essayée à la fermeture, qui
attend la fin de la tâche ; le `Net` de la boucle est relu après un
`dial_heard` réussi, pour ne pas récrire par-dessus des valeurs plus
récentes ; l'annonce repart à chaque enregistrement (`…_seq`), et une
officine retirée perd l'adresse qu'elle avait annoncée.

**Revue d'ensemble (0.334.0).** Une revue de toute la couche depuis 0.315.0 :
- une **correction ne vaut que de l'auteur de ce qu'elle corrige**
  (`trusted_view`) — une officine appairée effaçait sinon une rupture
  d'une autre ;
- **qui a envoyé, c'est qui a signé** : ruptures, valeurs sourcées et
  versions prennent le nom de l'auteur du relevé (nom donné ici, sinon
  celui qu'il signe), jamais celui que la charge utile déclare ;
- la porte ne lit **aucun journal avant la poignée de main** (clés et
  membres seulement ; `drive_any` charge celui du réseau choisi), une
  conversation à la fois par adresse et six par minute ;
- une adresse apprise ou annoncée est rangée à part (`net_reached:`),
  jamais à la place de l'adresse saisie, et l'appel aux officines
  appairées entendues ne dépend plus de la règle d'affichage « la
  première adresse tient » (un essai par minute et par officine, une
  adresse en échec écartée dix minutes ; une annonce venue de l'adresse et du port
  où on la connaît déjà passe outre la minute, un appel à la fois) ;
- une officine sans adresse saisie est appelée à `net_reached:` : une
  machine qui relaierait vers elle ne lit rien (la poignée de main exige
  sa clé), mais elle sait quand les deux officines se parlent ;
- un nom qu'une officine se donne et qu'une autre — ou celle-ci — porte
  déjà se lit **avec son empreinte** (`peer_label`), partout où elle n'a
  pas été nommée ici ; comparés sur leur squelette (lettres et chiffres),
  et un nom à parenthèse ou hors de l'alphabet latin la prend toujours ;
- la porte compte les coups par préfixe /64 en IPv6 ; l'invitation à une
  voisine et l'appel d'une officine ou d'un poste sont bornés en tout
  (`with_deadline`),
  et un appel qui tombe rend sa place.
- l'invitation faite à une voisine depuis la carte n'admet **qu'elle**
  (`Session::expecting`) : une autre est refusée avant tout code, et la
  porte reste ouverte pour la bonne ; de même pour « Rejoindre son
  réseau » ;
- une publication et une lecture à la fois par processus, le journal
  relu sous le verrou avant d'écrire ; officine et appartenance écrites
  d'un seul tenant ; IPv4 écrite en IPv6 jugée comme telle.

**Reste ouvert** : un identifiant (`<base>:<n>`) repris par une officine
appairée hostile garde sa place `INSERT OR IGNORE` là où il arrive le
premier — désormais **attribué à qui l'a signé**, mais la bonne ligne est
écartée. La parade (l'auteur rangé avec chaque ligne reçue, un identifiant
déjà tenu par un autre auteur refusé) demande une colonne de plus dans
deux tables répliquées : à décider.

**Reste ouvert, de même** : le nom de l'expéditeur est choisi à la
réception et rangé avec la ligne. Une officine qui signe un nom avant que
celle qu'elle imite ne soit connue ici le garde sur ces lignes. La parade
— l'auteur rangé avec chaque ligne, le nom lu à l'affichage — tient à la
même colonne.

**L'horloge ne se règle que sur les auteurs de confiance** (0.321.0) :
`Journal::trust` — cette officine et les siennes — et un rang venu d'un
autre (relayé par une officine appairée) ne déplace plus celui des
écritures d'ici. Ce qui restait ouvert plus haut est fermé.

Les invitations faites **à une officine voisine** depuis la carte (vue
par son annonce UDP `BPMOFFICINE1`, nom déclaré par elle-même) n'ont pas
de ticket, sont les seules annoncées sur le réseau local, et passent
par la comparaison des cinq groupes.

**L'échange**, ensuite : les têtes, puis des tours. À chaque tour les
deux postes envoient un `Want` — *ce qu'il me manque et ce que j'ai* —,
répondent à celui de l'autre par les enregistrements qui lui manquent,
et ferment par `EndRound`. Un tour où les deux demandes étaient vides
est la fin : les deux journaux sont d'accord, et les deux le savent.

Deux détails qui ne sont pas des détails :

* Le `Want` porte **les deux moitiés**, si bien qu'une première
  synchronisation tient en un tour au lieu d'un tour par génération du
  graphe.
* `EndRound` porte un drapeau `more`. C'est la différence entre « tu as
  tout ce que j'ai » et « je me suis arrêté au plafond ». Sans lui, le
  demandeur ne peut pas distinguer un enregistrement que le pair n'a pas
  d'un qu'il n'a pas encore envoyé : il doit alors soit abandonner tôt —
  en perdant des enregistrements, en silence —, soit demander pour
  toujours. On le lui dit.

---

## 6. Télémétrie : définition et limites

`meter.rs` tient des nombres : combien de conversations, combien
d'enregistrements ont traversé, combien d'octets, combien de refus et de
quelle sorte. Il existe parce qu'une synchronisation qui échoue en
silence est pire qu'une qui échoue bruyamment — quelqu'un doit pouvoir
ouvrir un volet et voir que ce poste n'a pas parlé à l'autre depuis
mardi.

**C'est un volet, pas une balise.** Rien n'y envoie rien nulle part. Pas
d'adresse, pas d'identifiant d'installation, pas de « statistiques
d'usage », et il n'y en aura pas : c'est une application qui tient des
données de santé, et la lecture honnête du mot télémétrie dans ce
cadre-là est *ce que l'officine peut voir de ses propres machines*.

**Elle compte, elle ne cite jamais.** Aucun champ n'est une chaîne. Un
message porte un fragment de ce dont il parle — un nom, un chemin, une
ligne — et l'endroit où personne ne cherche une fuite est un journal
que quelqu'un a allumé pour déboguer autre chose. Un refus est donc une
`Error`, une énumération sans rien dedans, et la phrase française est
écrite par l'écran qui montre le nombre.

`telemetry_counts_and_never_quotes_and_never_leaves` lit le texte du
module et refuse le jour où l'un ou l'autre change.

---

## 7. Intégration dans l'application (0.277)

Les décisions que cette section laissait à l'officine ont été prises
avec elle le 23/09/2026 ; ce qui suit est ce qui est construit, et où.

### 7.1 Où vivent les clés

Dans la base chiffrée, table `sync_local`, **qui ne voyage jamais** : la
graine du poste (`device`), la clé des postes (`trousseau`), le numéro
du poste (`post`), les empreintes des déclencheurs. La liste des postes,
elle, voyage (`sync_posts` : numéro, identité, nom, arrivée, retrait) —
chaque poste sait quels postes existent. Le réseau d'officines garde sa
propre clé dans `settings` (`net_trousseau`), qui ne scelle que
`Stream::Reseau` : les deux clés n'ouvrent rien l'une de l'autre, et un
test le vérifie sur les octets (`the_posts_key_and_the_network_key_open_nothing_of_each_other`).

### 7.2 Ligne ou opération : l'opération, champ par champ

`src/replica.rs` tient la liste des tables qui voyagent (`TABLES`, avec
leur flux) et de celles qui restent (`LOCAL`, avec la raison) ; un test
lit le schéma des deux fichiers et échoue sur une table rangée nulle part
(`every_table_either_travels_or_stays_and_none_is_forgotten`).

Une écriture voyage comme une opération : création (la ligne entière),
modification (**les seuls champs changés**, avec leur ancienne valeur),
suppression (la ligne vue). Reçue, une modification ne s'applique que
sur la valeur qu'elle remplaçait, champ par champ ; sinon elle attend
« à arbitrer » (`sync_conflicts`), et « Garder la mienne » renvoie la
valeur locale comme remplaçant l'autre, ce qui remet les postes
d'accord. Le registre, `stup_labels`, `stup_numbers`, la caisse, le
journal des accès et les journaux des ruptures et des versions sont **en
ajout seul** : une
modification reçue pour eux est refusée, et le refus se voit.

Deux postes ne sont pas toujours à la même version : le lanceur les met
à jour l'un après l'autre. Une colonne qu'un poste plus récent écrit et
que celui-ci n'a pas **ne se compare ni ne s'écrit** (`decide`, et déjà
la création et la modification appliquées) : comptée, elle faisait de
chaque effacement venu de lui une question, et de chaque écriture qui la
touchait une question que « Prendre la leur » ne pouvait pas régler
(0.390).

### 7.3 La projection

**Capturée par des déclencheurs** posés par `Db::install_capture` —
seulement sur une base qui fait partie d'un groupe : hors groupe, aucun
déclencheur, aucun coût, aucun changement. Chaque écriture d'une table
qui voyage laisse sa ligne avant et après dans `sync_log` (un par
fichier), dans la transaction qui écrit ; `Postes::publish` en fait des
lots de moins de quarante mille octets, scellés par flux. Ranger ce qui
vient d'ailleurs se fait **sans capture** (`sync_mute`, dans la
transaction qui range), si bien qu'une écriture ne revient jamais à son
poste. Les migrations aussi sont muettes : chaque poste fait les
siennes.

**Les numéros viennent d'un bloc par poste** (`sync_blocks`,
`db::next_id`) : chaque `INSERT` d'une table numérotée tire le suivant du
plus haut numéro du bloc de ce poste — jamais celui d'un autre, jamais un
numéro supprimé. Les dossiers ont des blocs de cent mille lisibles (le
fondateur garde les siens), le reste des blocs d'un million de millions.
`replica_inserts_draw_from_the_block` lit `db.rs` et refuse un `INSERT`
qui oublierait son `id`.

**Les numéros d'ordonnancier viennent du logiciel de l'officine** : tapés
sur chaque délivrance, sur n'importe quel poste, ils voyagent avec la
ligne. Aucun poste n'en attribue (`stup_numbers`, en ajout seul, ne garde
que ceux d'avant). Le poste de référence — réglage `sync_reference` qui
voyage — est le seul à semer le contenu livré.

### 7.4 L'écran

Options › Base › « Postes de l'officine… » : l'empreinte du poste, le
groupe, les postes (nom, référence, retrait), ce qui attend d'être
arbitré, et les gestes — inviter (la première invitation **fonde** le
groupe, `Job::Seed`, depuis la 0.434), rejoindre (qui **remplace** ce que le
poste contenait), inviter, synchroniser, quitter. La réserve est écrite
avant le contenu.

### 7.5 Le transport

Trois chemins (`src/postes.rs`) : **automatique sur le réseau local**
tant que l'application est ouverte — une porte tenue ouverte aux postes
du groupe, une annonce UDP toutes les trois secondes qui ne porte que
l'identité et le port, une conversation dès qu'une écriture est
capturée ; un **dossier d'échange** (`<empreinte>.bpmposte`) ; des
**adresses écrites** ; plus le bouton et la fermeture. `link.rs` ne
décide toujours de rien : c'est l'application qui a un fil.

**Relier un poste** (0.343) : l'invitation ouverte s'annonce toutes les
deux secondes, le temps qu'elle attend — `BPMPOSTE2 <poste> <port>
<groupe> pour:<poste invité|*> <étiquette>` (`beacon_invite`). Rien de
secret : l'étiquette est 4 octets de `derive_key("bpm-caddy/invite-tag/v1",
ticket)`, qui sert au poste invité à choisir, parmi les invitations
entendues, **celle que son code ouvre** (`join_target`) — une annonce
contrefaite ne la porte pas et ne détourne pas le code vers une autre
adresse. Le code seul suffit alors ; la poignée de main reste celle du
ticket (preuve des deux côtés, `session::proved`). L'invitation accepte
vingt connexions, trois par adresse, chacune lâchée après huit secondes
de silence : une annonce publique ne donne plus à qui la lit le moyen de
la fermer d'un essai. La liste des postes entendus garde la première
adresse d'une identité tant qu'elle s'annonce, et 64 postes au plus.

**L'officine** (`settings`, clé `pharmacy`, un bloc TOML) se fusionne
champ par champ (`PharmacyConfig::merge`) : à l'enregistrement refusé
(`Session::save_officine`) et à la réception (`replica::merge_officine`,
qui ne fusionne pas une valeur portant un champ que cette version ne
connaît pas). Un même champ changé des deux côtés reste un conflit.

Réseau d'officines, contenu versionné : fiches, codex, protocoles et,
depuis la 0.295, **lignes du TROD** (`versions::Kind::Trod`, nom de
voyage `protocole · nom`, champs `versions::TROD_FIELDS` — tout sauf le
rang) et **vaccins du catalogue** (`Kind::Vaccin`, champs `label`,
`code` et `schedule`). Ces deux-là voyagent sous une **identité figée**
(`origin`) : le nom livré pour une entrée livrée, sinon le nom porté à
la première publication. Le nom affiché est un champ comme un autre — un
renommage est une version, pas une entrée de plus chez les autres. Une
entrée livrée absente chez celle qui reçoit (retirée par elle) n'est pas
recréée ; deux entrées de même identité ne sont versionnées qu'une fois. Une ligne reçue inconnue est créée dans un protocole que
l'application connaît, jamais dans un protocole inventé.

Depuis « En ligne » (0.413), trois sortes de plus : **dispositifs**
(`Kind::Dispositif`, `DISPOSITIF_FIELDS`, par leur nom), **listes de
contrôle** (`Kind::Liste` : `subject` et `lignes`, les lignes entières
sous forme canonique `versions::items_json`, appliquées seulement sur la
liste qu'elles remplacent, comme un arbre de protocole) et **cascades**
(`Kind::Cascade` : `texte`, reconnues par le titre qu'il porte ; une
cascade reçue se crée par son texte, et une cascade livrée retirée ici
ne revient pas d'une version reçue). Une liste de contrôle porte la
façon de faire de l'officine, une **phrase imprimée** réécrite
(`Kind::Texte`, champ `value`, sous son adresse `content::key`, origine
le texte livré, écrite par `set_content` contre ce qui s'imprimait) son
nom ou son téléphone : leurs versions ne partent que si
`NetShares::listes` est coché (non par défaut, colonne `share_listes`),
et elles **ne se versionnent même pas** tant qu'aucun réseau ne les
partage — sauf une à une, cochées « Publier sur la carte »
(`published_entries`) : au journal, elles iraient aux autres postes, et
un poste d'une version d'avant la 0.413 (qui lit une sorte inconnue comme
une fiche) les publierait — d'où : tous les postes à jour avant de
cocher. Cochées, elles partent telles qu'elles sont ce jour-là. Pour
la même raison, les étiquettes et les sources d'un dispositif voyagent
sous `etiquettes` et `references` (`versions::dispositif_column`) : sous
`tags` et `sources`, une version d'avant les publiait comme ceux d'une
fiche, et les autres officines créaient une fiche au nom du pansement.
Un texte de cascade livré avant la relecture de la 0.360.0
(`shipped::CASCADES_REWORDED`) est rafraîchi s'il n'a pas été touché et
ne se versionne jamais ; un dispositif livré retiré ici ne revient pas
d'une version reçue. **Une liste ou une phrase reçue ne s'applique jamais
d'elle-même** (`receive_card_edits` la range au journal seulement) :
elle porterait les codes ou le nom et le téléphone de l'autre officine ;
« En ligne » l'offre, et l'historique la propose à adopter même posée sur
la valeur qu'elle remplace (`versions::pending_of`). Le texte livré d'une
phrase voyage **vide** — une marque, non le texte mot pour mot, que deux
versions de l'application ne livrent pas toujours pareil. Limite connue :
une liste ou une phrase déjà versionnée, puis décochée, reste au journal
des postes, et un poste d'une version antérieure à la 0.413 la publierait
encore (les autres officines la refusent au décodage, mais elle est
scellée dans le journal du réseau) : mettre tous les postes à jour avant
de cocher.

Depuis la 0.414, dispositifs, listes et cascades voyagent sous une
**identité figée** comme les vaccins (`origin`, figée par
`freeze_origins` à la première publication : le nom, le titre, ou le
titre que le texte d'une cascade porte) ; le nom d'un dispositif (`nom`)
et le titre d'une liste (`titre`) sont des champs comme les autres — un
renommage est une version, pas une entrée de plus chez les autres. « En ligne »
(`src/enligne.rs`) lit le journal reçu en offres par sorte ;
`Db::adopt_offer` prend une offre d'un geste — une entrée livrée retirée
ici revient alors entière (`entry_origin`), la dernière version
par-dessus, et une fiche renommée s'offre sous son nouveau nom.

Réseau d'officines (`src/network.rs`) : chaque conversation directe
est notée sur la ligne de l'officine (`net_peers.last_try`, `last_ok`,
`last_error` — la raison en français, jamais l'erreur brute du lien), et
chaque lecture du journal note ce que l'officine a envoyé : le nom sous
lequel elle signe (`seen_as`, le champ `officine` de ses charges), le
compte de ses enregistrements (`received`) et, quand ce compte monte,
l'heure des dernières nouvelles (`last_heard`). Les heures viennent de
SQLite (`strftime(…, 'localtime')`) : aucune horloge dans le module. Ces
colonnes sont locales au poste, comme la table.

### 7.6 Données non répliquées

Les pièces scannées (§ 7.6 d'origine : des fichiers, pas des
enregistrements), la télémétrie du poste, le journal du réseau
d'officines et ce que ce poste en a déjà envoyé, et tout `sync_*` propre
au poste.

### 7.6 bis Plusieurs réseaux

Le réseau principal garde son rangement d'origine (`settings.net_trousseau`,
`net_records` à `network` vide, clés de `net_published` telles quelles) :
rien n'a été migré. Les réseaux de plus vivent dans `net_networks`
(empreinte de la clé, clé, nom, trois interrupteurs de partage ; voyage
entre les postes comme `settings`, écrit contre ce que l'écran montrait) ;
le dossier d'échange de chacun, propre au poste, dans `net_folders` ; qui
est de quel réseau, dans `net_memberships` (local, comme `net_peers`, avec
un compte d'enregistrements par réseau) — une officine sans ligne est du
principal. Rejoindre un réseau de plus se fait sur un journal vide (rien
du principal ne passe à celle qui invite) ; rejoindre un réseau dont on
est déjà ne range rien. Chaque réseau a son journal ; ce qui est parti se note
`<réseau>|<clé>`. Un dossier d'échange peut servir à plusieurs réseaux :
chacun y lit tous les fichiers `.bpmnet` et ne garde que les
enregistrements **que sa clé ouvre**. L'identité de l'appareil et sa clé
de boîte sont les mêmes partout. Un message part par chaque réseau qui
compte un de ses destinataires, scellé pour ceux de ce réseau.

### 7.6 ter Le réseau ouvert (2026-10)

Un réseau de plus, que toute officine rejoint d'elle-même (`network::join_open`,
au déverrouillage du poste de référence) sauf quand elle l'a quitté
(`settings.net_ouvert_quitte`). Sa clé est **publique** : tirée par
`blake3::derive_key` d'un texte fixe, la même partout. Elle ne protège
donc pas ce qui y circule des autres officines — ce n'est pas son rôle :
y circule ce qu'une officine dit à toutes. Ce qui protège :

- **Rien ne s'applique de soi-même.** `Net::absorb` range les versions par
  `receive_card_edits_offered` : au journal des versions, proposées par
  « En ligne », jamais écrites dans une fiche. Les relais offerts et
  demandés n'y sont pas lus.
- **Seule l'officine règle son horloge** (`Journal::trust([moi])`) : un rang
  de Lamport au plafond, d'où qu'il vienne, ne brouille pas l'ordre.
- **Une correction ne vaut que de son auteur** (`trusted_view`, la règle des
  autres réseaux).
- **Un budget par inconnue** (`Journal::limit_strangers`, `OPEN_LIMITS`) :
  1 Mo par auteur, 64 Mo en tout ; un enregistrement de trop est refusé et
  n'est plus demandé. Les auteurs de confiance — l'officine elle-même — ne
  comptent pas.
- **La session ne s'ouvre qu'à la synchronisation** (`Session::open_to_all`,
  `Session::answer_any_open`) : jamais à une invitation ni à un appairage.
  Un réseau fermé, à la même porte, refuse toujours l'inconnue.
- **Une officine écartée n'est plus lue**, et ne s'inscrit plus.
- Une officine d'un autre réseau de l'officine est lue là, pas au réseau
  ouvert : pas deux fois, et sous le nom qu'on lui a donné.
- **Seuls les membres sont lus** (au plus `MOST_OPEN_MEMBERS`) : au-delà,
  un auteur reste au journal, borné, sans écrire une ligne en base.
- **Une source porte l'empreinte** (« Nom (XXXX XXXX) »), et chaque
  identifiant reçu est préfixé de l'auteur (`o<16 hex>:`) : une inconnue
  ne signe pas du nom d'une autre, ne prend pas la place d'un numéro
  d'une autre — ni de celle-ci —, ne retire pas ce qu'elle n'a pas écrit.
- **Ce qui y part est dépouillé** : ni note ni initiales aux ruptures,
  pas d'initiales aux versions, ni listes de contrôle, ni adresse, ni
  relais. Une adresse annoncée n'y est pas lue : on ne compose que là où
  l'officine a déjà répondu.

Les officines qui y parlent sont inscrites (`add_open_member`, au plus
`MOST_OPEN_MEMBERS`) : celle qui frappe, celle qu'on joint, l'auteur d'un
enregistrement relayé. Sur le réseau local, toute officine qui s'annonce
est composée (`dial_open`) et n'est inscrite qu'une fois qu'elle a répondu
sous l'identité annoncée. Une synchronisation compose au plus
`OPEN_DIALS` officines du réseau ouvert, les plus récemment jointes
d'abord. Pas de dossier d'échange pour lui.

**Au-delà du réseau local : la DHT** (`src/dht.rs`). Le poste de
référence demande au routeur d'ouvrir la porte des officines (UPnP,
`reach::open_officine_mapping`) et, s'il l'obtient, s'annonce sous un
sujet fixe de la DHT publique de BitTorrent ; toutes les demi-heures, il
y cherche les autres et en compose au plus `dht::MOST_DIALS`, chacune au
plus une fois par six heures (`network::dial_found` : qui répond est
inscrite, sauf soi-même). Une officine sans porte ouverte compose quand
même : une conversation va dans les deux sens. La DHT n'apprend qu'une
adresse IPv4 sous ce sujet ; une adresse trouvée ne prouve rien, la
conversation dit qui répond, et ce qu'elle apporte est lu comme tout le
réseau ouvert. **Aucun serveur** : ni l'éditeur, ni une officine.

**La liste ne se remplit pas pour de bon.** Les auteurs seulement relayés
n'ont droit qu'à la moitié de `MOST_OPEN_MEMBERS` ; l'autre reste aux
officines jointes ou qui ont frappé. Une officine muette depuis
`OPEN_FORGET_DAYS` (90 jours) sort de la liste, et ses enregistrements
encore relayés ne l'y remettent pas — une conversation directe, si. Les
officines **rencontrées** (une conversation directe réussie) ont leur
budget au journal, mais **le total ne les lie pas** (`Journal::favour`) :
des identités jetables qui l'auraient rempli ne ferment pas la porte aux
officines rencontrées ensuite. Au pire, 1 Mo pour chacune des 256, en plus
des 64 Mo des autres. Rencontrée pour la première fois quand le total est
plein, une officine est composée une seconde fois aussitôt, favorisée.

### 7.7 La boîte scellée : un message pour quelques officines

Le trousseau du réseau ouvre tout ce que le réseau transporte, et toutes
les officines membres le tiennent. Un message pour deux officines sur
dix passe donc sous un **second sceau**, `bpm_sync::boxed` :

* chaque appareil a une **clé de boîte** X25519, troisième dérivation de
  sa graine (`Device::box_secret`, domaine `bpm-caddy/device-box/v1`) —
  jamais la clé de signature ;
* chaque officine **annonce** la partie publique une fois dans le journal
  (`{"t":"boite","k":…}`) ; la signature de l'enregistrement dit qui
  l'annonce, et `net_peers.box_key` la garde ;
* un message est scellé une fois sous une clé de contenu tirée au hasard
  (XChaCha20-Poly1305), elle-même scellée pour chaque destinataire sous
  une clé dérivée (BLAKE3 `derive_key`) d'un échange X25519 avec une clé
  jetable ; l'en-tête est lié au contenu ;
* la boîte **ne nomme pas** ses destinataires : chacun essaie les
  entrées ; l'expéditrice s'y met aussi, pour se relire ;
* dans le journal, la charge commence par `BOX1` : les lecteurs JSON
  (ruptures, versions, valeurs) la laissent passer.

Un **fichier joint** voyage en morceaux de 12 Kio (en hex, dans la table
`message_chunks` entre postes, un morceau par boîte entre officines),
décrit par `message_files` (nom, taille, empreinte BLAKE3, nombre de
morceaux). Il ne s'enregistre qu'entier et conforme à son empreinte ;
5 Mio au plus, parce que chaque morceau reste au journal de chaque poste
et de chaque officine du réseau.

Un message qui nomme un patient ne part qu'après confirmation, et
l'envoi est écrit au journal des accès (`transmis`). Un message dont une
destinataire n'a pas encore annoncé sa clé attend la synchronisation
suivante. Deux postes qui absorbent la même boîte avant de se
synchroniser entre eux peuvent l'écrire deux fois ; l'`uid` évite tout
autre doublon.

### 7.8 Le compagnon : le téléphone d'un membre de l'équipe

Décidé avec l'officine le 26/09/2026 : un téléphone Android personnel,
par personne, rejoint le groupe des postes comme **compagnon**. Il lit
les fiches, l'agenda, le planning et la messagerie d'équipe ; il écrit
à l'équipe. Il ne tient jamais de quoi ouvrir un dossier.

**Une part de la clé, jamais la clé.** La clé des postes (`Trousseau`)
dérive une clé par flux ; un compagnon reçoit, au lieu du secret, une
**part** (`Trousseau::share`) : le nom du groupe et les clés de quatre
flux, `postes::COMPANION` — `Fiches`, `Agenda`, `Planning`, `Equipe`.
Il ne peut rien dériver d'autre : il n'y a pas de secret dans une part.
La part passe par la même trame chiffrée que la clé d'un poste, au même
moment, après la même preuve (`Frame::Share`, `Session::sharing`).
**Le code d'invitation dit à quelle sorte de poste il est destiné** : un
compagnon prouve le ticket sous un rôle à lui (`ROLE_JOIN_COMPANION`,
`Session::as_companion`), qu'une invitation de poste n'accepte pas — et
inversement. Un téléphone qui saisit le code d'un poste ne reçoit donc
rien, pas même une clé entière qu'il refuserait ensuite ; un poste qui
saisit celui d'un téléphone non plus, et l'invitation reste ouverte pour
celui à qui elle était destinée. Le poste montre aussi le code complet
(ticket et adresse) en QR, que le téléphone lit avec le lecteur des
services Google (sans autorisation caméra pour l'application).

**Écrire, c'est sceller sur le flux de la table.** Une écriture reçue
n'est rangée que si l'enregistrement qui la porte a été scellé sur le
flux de sa table (`replica::admissible`, appelé par `Db::apply_gated`) :
une ligne de patient, du registre ou des réglages glissée par un
téléphone modifié dans un enregistrement de l'équipe n'est pas rangée,
parce qu'il ne tient pas la clé qui scelle ces flux. Deux exceptions,
pour les postes d'avant : les tables de l'équipe arrivent aussi sous
`Dossiers` et `Officine`. Et sous le flux de l'équipe, un poste qui tient
la clé entière n'accepte pour `sync_posts` que ce qu'un poste dit de
lui-même — sa ligne à l'arrivée, son nom et ses adresses ensuite ; les
gestes sur les autres postes (retirer, nommer) voyagent **aussi** sous
`Officine`, que seuls les postes de bureau scellent. Ce qu'un téléphone
modifié peut encore écrire : les fiches, l'agenda, le planning et la
messagerie — ses quatre flux, comme un poste de bureau. L'application
ne lui propose pas d'écrire le planning ; rien d'autre ne l'en empêche.

**Il tient tout, il n'ouvre que sa part.** Le journal est un graphe :
un enregistrement nomme ses parents, de tous les flux. Le compagnon
reçoit donc tous les enregistrements — scellés — pour que son graphe
reste entier, et les relaie comme un poste d'une version antérieure
relaie un flux qu'il ne connaît pas (`Stream::Autre`). Il ne range que
ce que sa part ouvre (`Posts::absorb`) et ne scelle que dans ses flux
(`Posts::seal` laisse le reste sur le téléphone). Un téléphone perdu
porte des dossiers chiffrés sous une clé qu'il n'a pas, dans une base
chiffrée sous une clé du Keystore que seul le verrouillage de l'écran
ouvre.

**Le flux de l'équipe.** La messagerie voyageait sous `Dossiers` (un
message peut citer un patient) et la liste des postes sous `Officine`
(qui porte aussi la clé du réseau d'officines). Les deux passent sur un
flux à eux, `Equipe` (code 9) : `conversations`, `messages`,
`message_reads`, `message_files`, `message_chunks`, `sync_posts`. Un
message cite un patient par son numéro, jamais par son dossier ; le
téléphone l'indique et ne l'ouvre pas. **Ce que le téléphone reçoit
aussi**, et qu'il faut savoir : les fichiers joints aux messages
d'équipe (une pièce jointe peut être un document de patient — c'est
alors l'équipe qui l'a envoyé), et les conversations avec les autres
officines (rangées dans les mêmes tables ; l'application ne les montre
pas).

La liste des postes et les conversations qui existaient avant, un poste
les scelle à nouveau une fois, avant sa première invitation d'un
compagnon (`Posts::reseal_team`). Pas les messages d'avant : le
téléphone reçoit ceux écrits depuis.

**Mettre à jour tous les postes.** Un poste d'une version antérieure
garde les enregistrements de l'équipe sans les lire : il ne voit pas
les nouveaux messages, ni un poste relié par un poste à jour, ni un
retrait fait sur un poste à jour — sauf la liste des postes, qui voyage
aussi sous `Officine`. Le lanceur met chaque poste à jour à son
démarrage ; tant qu'un poste ne l'est pas, la messagerie le contourne.
Pendant ce temps, deux traces bénignes : une même conversation lue sur
un poste à jour puis sur un ancien donne une question « à arbitrer »
sur l'heure de lecture ; et un poste mis à jour tard relit la copie
`Equipe` des lignes de postes, ce qui peut poser la même question pour
un poste renommé entre-temps. Rien n'est perdu ; la question se ferme
en gardant l'une ou l'autre valeur.

**Le premier mot.** Un poste admis (compagnon ou non) range ce qu'il a
reçu, prend son numéro et le dit aussitôt à la porte qui l'a admis ;
elle l'attend une minute (`FIRST_WORD`). Sans cela, une seconde
invitation avant sa synchronisation suivante donnait le même numéro à
deux postes.

**Le transport.** Le téléphone est toujours celui qui compose : il
n'ouvre aucune porte et ne tourne pas en arrière-plan ; il synchronise à
l'ouverture, au retour à l'écran, chaque minute tant qu'il y reste, au
bouton et après chaque écriture. Il écoute les annonces
des postes sur le Wi-Fi (3,5 s) ; chaque poste du groupe est composé
une fois — à l'adresse où il s'est annoncé, sinon aux adresses qu'il
publie — puis viennent les adresses écrites dans les réglages du
téléphone.

**Joignable depuis Internet** (`[postes] internet`, Options › Base,
**éteint par défaut**, poste par poste). Un poste qui l'allume :

* ouvre une seconde porte, **IPv6 seulement** (`Door::open_v6_only`), à
  côté de celle du réseau local, sur le même port ;
* demande au routeur d'ouvrir ce port en IPv4 (UPnP IGD, bail d'une
  heure, renouvelé toutes les vingt minutes, refermé à la sortie) ;
* **publie** ses adresses composables dans sa ligne de `sync_posts`
  (colonne `reach`, flux `Equipe`) : l'adresse publique du routeur, et
  ses adresses IPv6 globales. Une adresse privée, ou derrière une
  traduction d'opérateur (`100.64.0.0/10`), n'est pas publiée
  (`reach.rs`) ; le téléphone relit la ligne avec la même méfiance.

C'est la seule porte que l'application tient ouverte au-delà du réseau
local. Elle ne change rien à la conversation : la poignée de main
chiffrée, puis une réponse aux seuls postes du groupe ; un inconnu
n'obtient pas un octet du journal. Ce qu'elle expose : l'existence d'un
poste BPM-Caddy à cette adresse, et une porte qu'un inconnu peut tenir
occupée — une conversation reçue dure au plus trois minutes
(`POSTS_ANSWER_BOUND`, ce qui est arrivé est gardé), et une adresse
d'Internet qui frappe plus de douze fois dans la minute est lâchée sans
conversation (`Knocks` ; le réseau local n'est pas compté, ni une
frappe devenue conversation avec un poste du groupe — plusieurs postes
d'un site derrière une seule adresse ne se ferment pas la porte). C'est la
synchronisation des postes qu'on peut ralentir ainsi, pas les données.
Le port ouvert par le routeur est aussi celui des invitations : une
invitation ouverte l'est alors depuis Internet, protégée par son code de
quatre-vingts bits et ses vingt essais. En IPv6, la box doit laisser
passer le port (certaines le refusent par défaut) ; derrière un CGNAT
sans IPv6, rien ne rend le poste joignable : le téléphone synchronise au
retour sur le Wi-Fi. Deux postes « joignables » d'une même officine sur
le même port se disputent l'ouverture du routeur : un seul par officine,
ou des ports différents.

**Le code.** `mobile/` (crate `bpm-caddy-mobile`, UniFFI) traduit la
bibliothèque — sans la fonction `desktop` — pour l'application Kotlin de
`android/`. Les libellés du téléphone sont dans `strings.fr.toml`
(`mobile_…`) ; un test du crate lit les sources Kotlin et refuse une
clé absente.

### 7.9 Le relais sans mémoire

Pour l'officine qu'aucun chemin ne rend joignable (CGNAT sans IPv6) :
une **officine amie du réseau** met en relation son poste et le
téléphone de son équipe, **sans rien lire ni rien garder**
(`src/relay.rs`). Deux réglages, éteints par défaut :

* **« Relayer les téléphones des officines du réseau »** (`[reseau]
  relais`, port `port_relais`, 7746) sur un poste de l'officine qui
  relaie — joignable, par UPnP ou un transfert dans sa box. Il annonce
  où au réseau (`{"t":"relais","a":…}`, retiré en l'éteignant) et
  n'accepte que les groupes de postes que des officines du réseau lui
  ont demandé de relayer.
* **« Se faire relayer par le réseau »** (`[postes] relais`) sur un
  poste de l'officine relayée. Il demande au réseau de relayer son
  groupe (`{"t":"relais_demande","g":…}`, le nom du groupe, rien
  d'autre), **appelle** chaque relais offert et y **attend** ; il publie
  ces relais dans sa ligne de `sync_posts` (`relais:hôte:port`), où les
  téléphones les lisent.

Le téléphone qui ne joint aucun poste ni sur le Wi-Fi, ni à ses
adresses, **appelle** un relais en disant le nom du groupe ; le relais
le met en relation avec un poste du groupe qui attend, et passe les
octets de l'un à l'autre (`BPMRELAIS1 attente|appel <groupe>`, puis
`ENTRE`). Ce qui passe est la conversation du § 5, chiffrée de bout en
bout : la poignée de main prouve de chaque côté qu'on parle au bon
poste, le relais n'a aucune clé. Il sait qu'un groupe échange, quand et
combien. Ses bornes : 256 connexions en tout ; la préface dite en cinq
secondes, pas un octet à la fois pendant dix minutes ; quatre attentes
par groupe, **refusées au-delà** — jamais mises à la place d'une autre ;
une attente de cinq minutes, que le poste renouvelle toutes les quatre
(une traduction d'adresse d'opérateur oublie une connexion muette) ;
trente appels par adresse et par minute ; chaque conversation dix
minutes, 256 Mio par sens, 90 s de silence des deux côtés (un relais
plein lâche d'abord les attentes échues). Le poste qui attend reçoit
une conversation relayée à la fois, au plus douze par minute, d'une
minute chacune : qui connaît le nom du groupe ne tient pas son fil. Un
inconnu qui se présenterait comme poste du groupe n'obtiendrait que
l'échec d'une poignée de main.

L'offre et la demande s'annoncent une fois **par numéro**
(`set_relay_value`), pas par valeur : un relais qui redémarre à la même
adresse est annoncé de nouveau. Chaque officine ne retient que le
dernier mot de chacune — une demande retirée ne laisse pas son groupe
accepté.

Ce qui n'est pas couvert par un test de bout en bout : le fil
automatique qui sert ou attend — un relais n'est accepté qu'à une
adresse publique, et servir demande au routeur un port. Le relais
lui-même, la conversation relayée et les enregistrements du réseau le
sont (`relay::tests`, `a_phone_reaches_its_desktop_through_a_relay`,
`a_relay_offer_and_a_relay_request_cross_the_network`). Un seul poste
par officine devrait relayer : l'offre est un réglage de l'officine.

### 7.10 Le répertoire des officines

Pour les officines qui ne se sont **pas appairées** : une carte où
paraît celle qui le veut, et son partage à parcourir
(`src/repertoire.rs`, la vue « Carte des officines »). Ce n'est pas le
réseau : rien ne s'y synchronise, rien n'entre de soi-même, et aucune
clé de réseau n'y passe.

* **Héberger** (`[reseau] repertoire`, `port_repertoire`, 7747) : un
  poste tient le répertoire, en mémoire seulement, en IPv6 et en IPv4,
  et demande au routeur d'ouvrir le port IPv4 ; il montre aussi ses
  adresses IPv6 publiques, où la box n'a qu'à laisser entrer. Aucune
  adresse n'est livrée : les officines écrivent celle qu'on leur a
  donnée.
* **Paraître** (réglage de l'officine, `settings`, clé `repertoire` :
  adresse, paraître, commune, FINESS — écrit contre ce qui était lu) :
  toutes les dix minutes, le poste ouvert envoie — composée et signée
  sur son fil — sa **carte de visite**
  signée par la clé de l'officine sur le réseau (la même que
  l'appairage montre) et son **catalogue** — les seuls champs des sortes
  publiques dont la dernière version est d'ici (`enligne::own_offers`),
  **et que la base tient encore** (`repertoire::catalogue_where` : le
  journal garde les versions d'une entrée supprimée, qui ne se publie
  plus), **et qu'elle a cochée « Publier sur la carte »** (table
  `published_entries`, sorte et clé, qui voyage entre les postes sur le
  flux Officine ; lue par `App::cards_here`) — et, si elle l'a choisi
  (`Presence::ruptures`), ses ruptures et substitutions récentes en
  articles de sorte `rupture` (`repertoire::supply_articles`,
  `ruptures::public_events` : les siennes, sans initiales ; lues au nom
  de la carte signée par `supply_of`, qu'une version d'avant écarte
  article par article) :
  jamais une liste ni une phrase qu'elle n'a pas cochée (cochées, elles
  voyagent sous `listepublique` et `textepublic`, `repertoire::wire_key`,
  qu'une version d'avant écarte article par article), ce qu'une officine
  appairée a réécrit, un patient, ni un nom qu'elle a reçu. Une rupture
  y part sans initiales ni note, jamais un retrait ; prise sur la carte,
  elle entre au journal sous `carte:<clé>:<uid>` (`taken_uid`) pour
  qu'un identifiant copié ne prenne pas la place de l'événement d'une
  officine appairée. Avant chaque annonce, ce qui a été écrit ou coché
  passe en version (`version_shared_entries`) : sans réseau appairé,
  rien d'autre ne versionne une préparation ou une liste. Décochée, ou l'adresse changée,
  une carte de départ la retire. Le réglage retient le répertoire qui a
  **reçu** la dernière carte (`parue`, `parue_le`) : un départ manqué,
  un redémarrage ou un autre poste le rattrape, sans jamais retenir la
  carte destinée au nouveau répertoire, et rien n'est plus dû passé un
  jour — le répertoire a oublié la carte.
* **Lire** : la liste des cartes — à l'ouverture de la carte, et
  d'elle-même toutes les dix minutes tant qu'une adresse est écrite,
  **même sans paraître** (le répertoire voit alors l'adresse du poste,
  comme il la voit à chaque lecture) ; l'onglet compte celles qui sont
  apparues depuis la dernière visite, relevées par base et par
  répertoire — puis le catalogue d'une officine,
  vérifiés **par chaque poste** — la signature, l'empreinte du
  catalogue, et ce qui peut entrer (`versions::admissible`, sortes
  publiques connues). L'officine qui héberge ne peut ni changer un mot,
  ni parler à la place d'une autre ; « ouverte » se lit à l'heure de la
  carte signée, contre l'horloge de qui lit. Le FINESS déclaré se lit
  au registre livré (`src/finess.rs`, l'extraction publique réduite aux
  officines) : au registre à la commune dite, sous un autre nom,
  ailleurs, ou absent — signalé, jamais refusé, et jamais une preuve.
* **Balayer** (« Tout le répertoire… ») : la liste, puis au plus
  `SWEEP_MOST` (20) catalogues — sous les trente appels par minute que
  le répertoire tient, avec de quoi pour l'annonce et la relecture de la
  liste —, un balayage par
  minute (`SWEEP_EVERY`, comptée depuis la fin du précédent), 90 secondes en tout (`SWEEP_BUDGET`), arrêté
  à deux refus de suite. Les ouvertes d'abord (l'heure d'une carte bornée
  à maintenant), celles qui n'ont pas répondu en dernier ; chaque
  catalogue vérifié comme ci-dessus. Le poste les garde en mémoire sous
  l'empreinte de la carte (relus quand elle change, oubliés quand
  l'officine quitte la liste, `SWEEP_BYTES` au plus, à un catalogue
  près : un balayage s'arrête de lire quand ce qu'il a lu les passe — les lus depuis le
  plus longtemps, à l'horloge d'ici, écartés d'abord, jamais ceux du
  balayage même) ; rien n'est écrit sur le disque. Une même entrée chez
  plusieurs officines est une offre à leurs noms.
* **Prendre leur version** d'une entrée qu'on a (`Session::take_theirs`,
  `Db::take_entry_values`) : une liste ou une phrase seulement d'un
  réseau qui les partage ou cochée par son auteur ; jamais une valeur
  que `versions::admissible` refuse, jamais le nom d'une fiche ; chaque
  champ qui diffère, tous ou aucun, contre ce que la base disait quand la
  ligne a été lue. Une version locale le porte (sans source : c'est
  désormais la version de cette officine, qui part vers son réseau et son
  catalogue).

Ses bornes : une carte de 4 Kio, un catalogue de 2 Mio **dans sa forme
exacte** (re-sérialisé, il doit redonner les mêmes octets, et le compte
par sorte de la carte doit être le sien). Le répertoire n'en juge que la
forme et les bornes, jamais ce qu'il dit : une sorte ou un champ qu'une
version d'après partagera passe chez un répertoire d'avant, et c'est
chaque poste qui écarte ce qu'il ne sait pas prendre — la même règle
(`repertoire::admitted`) que celle qui compose le catalogue. Le reste,
vérifié hors du verrou : 2 000 officines, 16 et 8 Mio
par adresse — comptés à chaque annonce, une carte qui change d'adresse
entre dans le compte de la nouvelle — et 256 Mio en tout ; 64
connexions dont 4 par adresse, 30 appels par adresse et par minute
(une table élaguée à la seconde, qui ne grossit pas sous les coups).
« Adresse » veut dire **abonné** (`repertoire::caller`) : une IPv4, ou
le préfixe `/64` d'une IPv6 — un abonné en tient des milliards et
passerait sinon sous chaque borne en changeant d'adresse. Un cran
au-dessus, le **réseau** (`repertoire::network` : un `/48`, ce qu'un
tunnel gratuit donne à une seule personne, ou un `/24`) a les siennes :
64 appelants dans la table, 64 cartes et 32 Mio, 16 connexions — ce
qu'aucune borne par préfixe ne ferme tout à fait : un abonné d'un `/56`
peut occuper pour une minute les places IPv6 de son `/48`, et qui tient
des dizaines de `/48` (cinq par compte de tunnel gratuit) peut encore
remplir le répertoire ; l'IPv4 reste ouverte aux voisins. Les adresses
IPv6 montrées à qui héberge écartent les liens coupés, les liens point à
point (un tunnel, un modem PPP — et donc aussi pour les téléphones des
postes), et sous Linux les adresses temporaires ou dépréciées ;
ailleurs, celles formées sur l'adresse matérielle passent d'abord. Deux
prises, chacune sa pile (l'IPv6 en `IPV6_V6ONLY` partout), et un port
tenu ailleurs, sur l'une ou l'autre, est une erreur plutôt qu'une
adresse donnée où répondrait un autre programme ;
cinq secondes pour dire sa préface, puis **une échéance unique** de
trente secondes pour la demande entière, lecture et écriture. Une carte n'est reçue que signée depuis moins d'une demi-heure
(et pas au-delà dans l'avenir) et plus récente que tout ce qu'on a vu de
cette clé, départ compris — retenu pour qui était là, et pour lui
seul : sauvée et rejouée, elle est refusée. Les
deux horloges doivent s'accorder à une demi-heure près.

Ce qu'il ne garantit pas : **qui est qui**. N'importe quelle clé peut
s'appeler « Pharmacie du Port » ; l'écran montre l'empreinte, et
l'appairage reste la seule preuve. L'officine qui héberge sait qui
paraît, d'où, et qui lit quoi.

### 7.11 La messagerie des patients

Décidée avec l'officine le 14/10/2026 (`docs/CHAT_PATIENT.md`) : un
patient écrit à l'officine depuis une page web, par un code QR qu'elle
lui remet. **Ce n'est pas le journal** : rien de ce qui est dit ici ne
passe par `bpm-sync`, et le patient n'entre dans aucun groupe. Trois
pièces :

* **la page** (`assets/patient/`), qui chiffre avec WebCrypto sous la
  clé du patient — 32 octets, dans le fragment du code, jamais envoyés à
  un serveur. **Publiée à part de la passerelle**, sur GitHub Pages
  (`.github/workflows/page.yml`, `patientchat::PAGE_SITE`), chaque
  version dans un dossier nommé par son empreinte (`page_hash`) : qui
  tient la passerelle ne peut pas servir une page qui lirait les
  messages avant de les chiffrer. Le lien du code nomme la passerelle
  (`p=` dans le fragment). Chaque poste compare toutes les six heures la
  page publiée, octet pour octet, à celle qu'il livre
  (`patientpost::check_page`) : un écart s'affiche en rouge dans
  l'onglet ;
* **la passerelle** (`src/passerelle.rs`, binaire `bpm-passerelle`),
  derrière un proxy TLS : un navigateur n'ouvre qu'un `wss://`, qui
  demande un domaine et un certificat qu'aucun poste n'a. Elle tient les
  postes qui attendent à la **boîte** d'une officine (`BPMPATIENT1
  attente <secret>` sur le port 7747, puis `ENTRE`), et passe les trames
  d'une page qui ouvre `/ws/<boîte>` à l'un d'eux. Le numéro de boîte
  est dans chaque code ; le secret, d'où il se tire, ne quitte pas les
  postes (`settings`, flux `Officine`) : qui a photographié un code ne
  peut pas attendre à la place de l'officine. Ses bornes : quatre
  attentes par boîte, refusées au-delà — une attente raccrochée est vue
  au premier coup d'œil et rend sa place ; seize attentes par adresse,
  toutes boîtes confondues ; une attente de cinq minutes, renouvelée à
  quatre ; **la page parle d'abord** — son bonjour en cinq secondes, avant
  qu'un poste soit pris : une page muette ne tient aucun poste ; une
  conversation d'une minute, et dix secondes par trame côté poste ;
  trente connexions par minute et par adresse sur le port des postes,
  deux cent quarante sur celui des pages (l'adresse que le proxy a vue),
  comptées à part ; seule une page de l'origine publiée obtient un
  WebSocket (`--origine` en ajoute une). Elle ne sert plus de page, et
  ne garde aucun message ; avec `--journal`, une ligne par connexion
  (l'heure, l'adresse, la boîte, l'issue — jamais une trame), un fichier
  par jour, effacé au bout de `--garde` jours, six mois au plus ;
* **le poste qui attend** (`src/patientpost.rs`) : chaque poste de
  bureau ouvert, deux attentes à la fois, une conversation à la fois,
  avec sa propre connexion à la base. Le compagnon n'attend jamais.

**Le protocole** (`src/patientchat.rs`) : la page dit bonjour avec un
nombre et un HMAC ; le poste essaie toutes les clés (toutes, quel que
soit celle qui répond : le temps ne dit pas où est le patient dans la
liste), répond `FERME` pour une clé fermée — **signé** sous la clé :
une passerelle ne peut pas fermer un patient de son propre chef — et
`INCONNU` pour une
inconnue — la page n'abandonne pas sur `INCONNU`, une clé donnée sur un
autre poste peut ne pas être encore arrivée sur celui qui répond. Sinon
il prouve qu'il tient la même clé, et chaque sens a sa clé AES-256-GCM
tirée par HKDF des deux nombres : une conversation rejouée ne s'ouvre
pas. Les réponses de l'officine s'échangent **par numéros déjà reçus**,
pas par « depuis » : chaque poste tire ses numéros de son bloc (§ 7.2),
et un numéro plus petit n'est pas un message plus ancien.

**Ce qui voyage entre les postes** : `patient_chat_keys` et
`patient_messages`, sous `Dossiers`, comme le dossier qu'ils suivent ;
supprimés avec lui. Un message renvoyé par la page après une coupure
n'est rangé qu'une fois sur un poste (`uid`) ; reçu par deux postes
avant qu'ils se soient vus, il est montré une fois.

### 7.12 Le poste en service (0.434)

Tout ce qui précède n'a lieu que **pendant que l'application tourne** :
la porte des postes, l'annonce, les conversations automatiques, le
réseau toutes les quinze minutes, l'attente à la passerelle des
patients. Jusqu'ici, le verrouillage (quinze minutes d'inactivité par
défaut) fermait la base et arrêtait tout, et fermer la fenêtre quittait :
un poste laissé le soir ne répondait plus ni aux patients ni aux autres
postes.

`[bureau] en_service` (par poste, **allumé par défaut** — décision de
l'officine : désactivable, non à activer) : le verrouillage voile
l'écran sans fermer la base, fermer la fenêtre la range dans la zone de
notification, et `[bureau] au_demarrage` lance l'application à
l'ouverture de session (`--arriere-plan`). Le détail est dans
`ARCHITECTURE.md`, « Le poste en service ». Ce que cela change ici :

- **La clé reste en mémoire** tant que le poste est en service, écran
  voilé compris : c'est ce qui lui permet de répondre. Le périmètre du
  § 2 ne bouge pas (un poste ouvert l'avait déjà) ; le temps, si.
  Décocher rend le verrouillage d'avant.
- **Aucune porte de plus.** Les postes, le réseau et la passerelle
  gardent leurs réglages ; le poste les tient simplement plus longtemps.
  `[postes] internet` et les relais restent éteints par défaut : ouvrir
  une porte sur Internet reste une décision de l'officine.
- **Le groupe se sème à la première invitation** (`Job::Seed`) et le
  réseau à la première « Inviter une officine… » : aucune clé n'est
  créée sans invitation, donc aucun poste seul ne se met à journaliser
  pour rien (le journal ne se compacte pas, § 8).

### 7.13 Les comptes (2026-10)

Un compte est une **personne** : sa clé tient en douze mots
(`src/compte.rs`). Ce qu'elle écrit est un enregistrement **signé par
le compte** — public en clair, partagé ou privé sous `boxed`, pour les
comptes choisis et pour elle. Ces enregistrements voyagent entre les
postes (`account_records`, flux Équipe, en ajout seul) et **dans le
journal de chaque réseau** de l'officine, préfixés `ACC1` : les
officines les gardent et les passent, réseau ouvert compris, sans
pouvoir lire ce qui est scellé ni rien réécrire. Un poste ne publie que
les comptes ouverts chez lui ; ce qui vient d'ailleurs circule par le
journal qui l'a apporté. Ils sont lus des auteurs que le réseau lit —
la signature du compte fait foi, pas celle du poste qui les a passés —
enregistrement par enregistrement, si bien qu'aucune « correction »
d'un autre ne les cache. La dernière version de chaque élément gagne,
au plus 200 éléments vivants par compte, 4 Kio chacun : le mégaoctet
que les autres officines accordent à chacune ne s'oublie pas. La phrase ne
quitte jamais le poste, qui la garde scellée sous le code de la
personne (Argon2id puis AES-256-GCM, table locale `account_keys`).

Les **billets** (vue Actualités) sont des éléments comme les autres,
publics ou partagés. Ce qu'un compte a **masqué ou bloqué** est un
élément de plus, scellé pour lui seul : il le suit d'un poste à
l'autre, et personne d'autre ne sait qui il a écarté. Écarter un
auteur est une lecture : ses enregistrements sont gardés et passés
comme avant.

**Jamais un patient** : ce qu'un compte range passe par les autres
officines, ce que § 8.4 refuse pour des données de santé. L'écran le
dit là où l'on écrit.

## 8. Travaux restants

1. **Le journal ne se compacte pas.** Chaque poste garde tout ce qui a
   été écrit depuis la fondation, et le relit en mémoire à chaque
   synchronisation. Une photographie datée, et l'oubli de ce qui la
   précède, viendront quand un groupe en aura besoin.
2. **Deux appairages simultanés** sur deux postes différents, hors
   ligne, donneraient le même numéro aux deux arrivants. Appairer un
   poste à la fois. (Deux appairages *successifs* au même poste ne le
   font plus : le premier mot, § 7.8.)
3. **L'ordre des lignes d'un même jour** se lit encore par numéro dans
   quelques vues ; entre deux postes, le bloc du second passe après celui
   du premier. Le registre des stupéfiants et l'historique des versions
   suivent désormais l'heure d'écriture (`created_at`, `card_edits.at`).
4. **Un relais qui garde** (une boîte chez une autre officine, pour
   échanger sans que les deux bouts soient en ligne en même temps) : pas
   construit, et pas sans décision — l'officine qui garderait, même
   chiffrées de bout en bout, les données de santé d'une autre ferait de
   l'hébergement pour compte de tiers, que le cadre HDS et le RGPD
   encadrent. Le relais construit (§ 7.9) ne garde rien.
   NAT-PMP / PCP non plus : UPnP seul.
5. **Le compagnon écrit l'équipe, l'agenda et les fiches**, par les
   chemins du bureau (comparer-et-écrire) ; le planning, il le lit
   seulement. Pas de fichier joint depuis le téléphone.
6. **Les comptes ne se compactent pas** : chaque version d'un élément
   reste une ligne d'`account_records`, et un enregistrement reçu par
   deux postes de l'officine y est rangé deux fois (lu une fois). Le
   compagnon les reçoit sans encore les montrer.
