# Messagerie patient par QR code

Demande : un QR code sur la fiche du patient, qu'il scanne pour écrire à
l'officine depuis une application ou un site, côté patient.

**Décidé le 14/10/2026 : le chemin A** — pas de serveur qui garde, le
poste allumé. **Le 16/10/2026 : sans serveur du tout** — la voie directe
(WebRTC, de la page au poste) est le chemin par défaut ; la passerelle
reste un second chemin facultatif, que l'officine tient ou choisit ; la
passerelle livrée (`BPM_CADDY_PASSERELLE`) n'est pas réglée. Ce document dit ce qui a été construit, ce
que cela demande, et ce qui reste ouvert.

## Ce qui change par rapport à aujourd'hui

Jusqu'ici, tout ce qui touche un patient restait dans la base chiffrée
de l'officine et sur ses postes (`replica::Flux::Dossiers`). Le
compagnon Android ne reçoit **jamais** un patient (`docs/SYNC.md`
§ 7.8), et ne reçoit pas non plus cette messagerie.

La messagerie ajoute un appareil qui n'est pas à l'officine (le
téléphone du patient), et une identité à prouver : ce patient, et pas un
autre. La clé du patient est cette preuve — qui tient le code tient la
conversation. D'où un code par patient, qu'on peut fermer et remplacer.

## Ce qui a été construit

**Côté patient : une page, pas une application.** Le code QR ouvre
`https://bpmcaddy.github.io/<empreinte>/#b=<boîte>&k=<clé>&f=<certificat>&a=<adresses>[&p=<passerelle>]`.
Le fragment (après `#`) n'est envoyé à personne par le navigateur : la
clé ne quitte pas le téléphone, et GitHub ne sait pas quelle officine
la page appelle.
La page (`assets/patient/`) est du HTML, du CSS et un script sans
bibliothèque ; elle chiffre avec WebCrypto. Une fois la clé gardée sur
le téléphone (`localStorage`), elle la retire de la barre d'adresse.
Un bandeau fixe, en tête, dit qu'elle n'est lue qu'aux heures
d'ouverture (15 ou 112 en cas d'urgence, un médecin pour un effet
indésirable grave) ; elle montre les heures d'ouverture de l'officine et
l'information due au patient (RGPD, art. 13 : responsable, finalité,
base légale, destinataires, durée, droits). Avant tout envoi, elle
vérifie que le navigateur chiffre comme le poste (des valeurs connues,
que `patientchat::tests` tient des deux côtés).

**La page est publiée à part de la passerelle**, sur GitHub Pages, depuis
le dépôt public (`.github/workflows/page.yml`), sous l'adresse d'une
organisation qui ne sert rien d'autre (`bpmCaddy`, dépôt
`bpmcaddy.github.io`, où le workflow pousse par une clé de déploiement
limitée à ce dépôt) : la page garde la clé et la conversation dans le
navigateur, que tout site de la même origine pourrait lire. Aucun autre
site ne doit jamais être publié dans cette organisation. Chaque version y a son
dossier, nommé par son empreinte, qui ne change plus une fois publié.
Qui tient la passerelle ne touche donc pas au code qui chiffre — c'était
la seule façon pour lui de lire un message. Chaque poste compare toutes
les six heures la page publiée à celle qu'il livre, octet pour octet
(`patientpost::check_page`) : un écart s'affiche en rouge dans l'onglet.

**Le protocole** (`src/patientchat.rs`) : quatre trames. La page prouve
qu'elle tient une clé (HMAC d'un nombre tiré au hasard) sans dire
laquelle ; le poste les essaie toutes, et prouve à son tour qu'il tient
la même. Chaque sens a sa clé (HKDF sur la clé du patient et les deux
nombres), AES-256-GCM. Une conversation enregistrée ne se rejoue pas.
La page envoie ce qui attendait sur le téléphone et les numéros des
réponses qu'elle a déjà ; le poste range, et rend ce qu'elle n'a pas.

**La voie directe** (`src/direct.rs`), le chemin par défaut, sans
serveur. Un navigateur ne parle pas TCP, et depuis une page en HTTPS il
n'ouvre de WebSocket que vers un nom qui a un certificat. WebRTC, lui,
joint une simple adresse, sans nom ni autorité de certification, pourvu
que chaque bout ait la description de l'autre — ce que d'habitude un
serveur de signalisation fait passer. Ici **le code QR est la
signalisation** : il porte l'empreinte du certificat DTLS de l'officine
(`f`, le même sur tous ses postes, un réglage de l'officine) et les
adresses de ses postes (`a`, six au plus) ; les identifiants ICE se
tirent du numéro de la boîte. La page écrit elle-même la réponse des
postes, chaque adresse en candidate : une seule connexion les essaie
toutes et garde la première qui répond. Le poste écoute en ICE-lite
(`str0m`, sans E/S, chiffrement en Rust pur), au port UDP **7748**, à ses
adresses IPv6 publiques et à son adresse IPv4 côté box, dont il demande
l'ouverture (UPnP) ; il range ce qu'il a trouvé dans un réglage de
l'officine par poste (`pchat_adr.<poste>`), et chaque réponse rend à la
page les adresses à jour : une officine dont l'adresse change reste
joignable de qui lui a écrit depuis. Le canal porte les mêmes trames
que la passerelle (une longueur, puis la trame), en morceaux de seize
kilo-octets. DTLS garantit en plus à la page qu'elle parle au poste de
l'officine (l'empreinte) ; le chiffrement des messages reste celui du
protocole. Essayé dans Firefox avec la vraie page
(`patientpost::tests::serve_directly_for_a_browser`, `--ignored`) :
tableau, conversation et envoi d'un message jusqu'à la base.

**Le tableau de l'officine** (`src/tableau.rs`, outil « Tableau de
l'officine », Ctrl+K « garde ») : ce qu'elle affiche pour tous, par
canal — la pharmacie de garde, les produits disponibles chez elle, ses
informations — chaque ligne avec ses dates. Un réglage de l'officine
(`tableau`), qui voyage entre ses postes. Les postes le servent par les
mêmes chemins, sans clé : la trame `BPMT1` seule, et le poste rend
l'objet JSON `tableau::Public` (`v`, `officine`, `adresse`, `telephone`,
`horaires`, `le`, `lignes` : `canal` `garde|produit|info`, `titre`,
`detail`, `du`, `au` en dates ISO) — c'est aussi la forme qu'un autre
logiciel peut lire. La page l'affiche au-dessus de la conversation ;
**le code du tableau** (`#b=…&t=1&f=…&a=…`, sans clé) l'ouvre seul, sur
un téléphone, en vitrine ou sur le site de l'officine ; avec `&e=1`, en
**écran** — en grand, relu chaque minute, gardant le dernier lu quand
l'officine ne répond pas : un tableau des départs pour la salle
d'attente, sur n'importe quel écran qui a un navigateur. Le 3237 (les
gardes, Résogardes) y est toujours rappelé.

**La passerelle** (`src/passerelle.rs`, binaire `bpm-passerelle`),
facultative : un second chemin, pour une officine dont aucun poste n'est
joignable depuis Internet. Pourquoi il en faut une alors : un navigateur ne parle pas TCP. Depuis une
page en HTTPS, il n'ouvre qu'un WebSocket chiffré (`wss://`), qui
demande un nom de domaine et un certificat — ce qu'un poste d'officine
n'a pas, et souvent il n'a même pas d'adresse joignable. Le relais des
officines (§ 7.9 de `SYNC.md`) parle TCP brut, sans certificat : il ne
peut pas servir une page. La passerelle se tient donc derrière un proxy
qui porte le certificat ; elle tient les postes qui attendent, et passe
les trames de l'un à l'autre sans les lire. Elle ne sert pas la page,
et n'accepte que les WebSocket de l'origine où elle est publiée. Elle ne
garde aucun message : sans poste qui attend, la page l'apprend
(fermeture 4004) et garde le message sur le téléphone. Elle tient, si on
le lui demande, un **journal des connexions** (`--journal`) : l'heure,
l'adresse IP, la boîte, l'issue — jamais une trame — un fichier par
jour, effacé au bout de `--garde` jours (trois mois sans rien dire, six
au plus).

**Côté officine** (`src/patientpost.rs`, onglet « Messagerie » du
dossier). Chaque poste de bureau ouvert écoute sur la voie directe et, si
une passerelle est réglée, y attend (deux attentes à la fois) ; il
répond avec la base. La clé et les messages
voyagent entre les postes avec les dossiers (`patient_chat_keys`,
`patient_messages`, flux `Dossiers`). Un code par dossier : en donner un
nouveau ferme le précédent ; fermer l'accès coupe le patient, et sa page
le lui dit.

## Les trois questions, et ce qui a été répondu

1. **Le chemin** : A.
2. **Le patient n'écrit qu'après un geste de l'officine** : le code
   qu'on lui remet (montré ou imprimé) est ce geste ; sans code, pas de
   messagerie. L'officine peut le fermer.
3. **Pièces jointes** : non pour l'instant — du texte seulement, deux
   mille caractères par message. Une photo d'ordonnance est une donnée
   de santé de plus ; la question reste ouverte, rien ne l'empêche
   plus tard.

## Le point réglementaire : l'avis d'un avocat (14/10/2026)

Deux échanges écrits avec un avocat, sur l'architecture décrite ici. Ce
qu'il en ressort, et ce que le logiciel en a fait :

* **HDS (art. L. 1111-8 CSP) : non**, pour la passerelle. Un
  acheminement en temps réel, sans écriture sur un support persistant et
  sans la clé, est du transport, pas de l'hébergement. Le point de
  friction était la page servie par la passerelle (son exploitant aurait
  pu la changer) : elle est désormais publiée à part, et vérifiée par
  les postes — « la réponse qu'attendrait la CNIL ».
* **Qui est qui (RGPD).** L'officine est responsable du traitement.
  L'exploitant de la passerelle est son **sous-traitant** (art. 28) : un
  contrat est obligatoire (une acceptation électronique dans le
  logiciel vaut, si on garde la preuve : le poste, l'heure, la version
  du texte accepté). Si l'éditeur ne fait que fournir le logiciel, il
  n'est pas sous-traitant ; une officine qui tient la passerelle pour
  d'autres devient leur sous-traitant. GitHub, qui publie la page, ne
  voit que des adresses IP : son contrat standard suffit.
* **AIPD** : une officine seule n'est pas « à grande échelle »
  (considérant 91), elle en est dispensée en principe. Un **dossier
  technique de sécurité** reste à lui fournir (description, nécessité,
  risques, mesures) — l'avocat propose d'en relire le plan.
* **Journaux** : pas de contenu, mais les métadonnées de connexion trois
  à six mois, pour la sécurité (art. 32) — d'où `--journal`.
* **Secret professionnel** : compatible, tant que seuls le patient et le
  pharmacien accèdent au clair. Mon espace santé est recommandé, pas
  obligatoire.
* **Base légale** : la prise en charge sanitaire (art. 9.2.h), sans
  consentement écrit ; l'information de l'article 13 sur la page.
* **Urgence** : une obligation de moyens ; le bandeau permanent de la
  page. Un délai de réponse de 24 à 48 h ouvrées est cité comme
  « recommandation ordinale » — **source à demander**.
* **Conservation** : avec le dossier du patient, et supprimée avec lui
  (c'est ce que fait la base). La durée de vingt ans citée — **source à
  demander** : c'est celle du dossier médical des établissements de
  santé (R. 1112-7 CSP), pas un texte propre à l'officine.
* **Droits du patient** : accès — une copie de la conversation,
  « Imprimer la conversation » ; effacement — l'officine peut refuser
  ce qui est nécessaire à la prise en charge (art. 17.3).

Reste à décider : **qui tient la passerelle** (l'éditeur, et il signe
un contrat de sous-traitance avec chaque officine ; ou une officine) et
les conditions d'utilisation qui vont avec.

## Mettre en service

**Par la voie directe**, rien d'autre que BPM-Caddy : onglet
« Messagerie » d'un dossier, « Régler la messagerie… », « Voie directe »
cochée (elle l'est d'emblée), Enregistrer. Le poste tire le certificat
de l'officine et le secret de sa boîte ; chaque poste en service écoute.
L'onglet dit si ce poste est joignable depuis Internet. S'il ne l'est
pas : une box sans UPnP s'ouvre à la main (port UDP 7748 vers un poste),
et son adresse publique s'écrit dans la même fenêtre (« Box ouverte à la
main ») — les codes la portent en tête ;
une ligne sans IPv6 et derrière une traduction d'opérateur ne se joint
pas — une passerelle reste alors le seul chemin. Puis, pour chaque
patient, « Donner l'accès ».

**Une passerelle, en plus** (facultative), se met en service ainsi :

1. **Une passerelle**, une fois pour toutes les officines qui s'en
   servent : un petit serveur, un nom de domaine, et un proxy qui porte
   le certificat. **`deploy/passerelle/`** fait tout cela d'un script
   (`install.sh` : service systemd durci, Caddy, pare-feu) ; le détail
   suit. Avec Caddy :

   ```text
   messagerie.example.fr {
       reverse_proxy 127.0.0.1:8746
   }
   ```

   (avec nginx, passer la mise à niveau WebSocket et l'adresse du
   patient, `X-Forwarded-For`), puis `bpm-passerelle` (construit par
   `cargo build --release --bin bpm-passerelle --no-default-features --features sync`) :

   ```text
   bpm-passerelle --journal /var/log/bpm-passerelle --garde 92
   ```

   Elle prend les WebSocket des pages sur `127.0.0.1:8746` et attend les
   postes sur le port **7747**, à ouvrir vers Internet. Elle ne sert pas
   la page (publiée sur GitHub Pages) : son exploitant ne peut pas lire
   les messages, mais il voit les adresses et les heures — il est le
   sous-traitant des officines qui s'en servent.
2. **Dans l'officine**, une fois : onglet « Messagerie » d'un dossier,
   « Régler la passerelle… », son nom de domaine. Le poste tire le secret
   de la boîte de l'officine ; le réglage vaut pour tous ses postes.
   **Ou rien** : une passerelle tenue par l'éditeur s'écrit dans les
   versions publiées (la variable `BPM_CADDY_PASSERELLE` du dépôt, que lit
   `release.yml`) ; tant que l'officine n'en règle aucune, c'est elle, et
   le poste de référence tire le secret de lui-même — la messagerie est
   prête à l'installation. Réglée vide, elle est éteinte.
   **Ses conditions** : `assets/passerelle-conditions.md`, vide tant
   que personne ne tient la passerelle livrée. Écrit (le contrat de
   sous-traitance, relu par l'avocat), il est livré avec la version, et
   la passerelle livrée ne sert l'officine qu'après son acceptation —
   sur n'importe quel poste, au nom de l'officine : le poste, le jour,
   l'heure, le nom et la version du texte (douze chiffres d'empreinte)
   sont rangés dans les réglages de l'officine
   (`passerelle_conditions`) et voyagent entre ses postes. Un texte
   changé se redemande sans couper la messagerie
   (`patientchat::terms_state`). **Le premier texte, lui, coupe** : une
   officine qui se servait de la passerelle livrée sans conditions attend,
   à la version qui les livre, qu'on les accepte — pas de traitement
   pour son compte sans contrat. À annoncer avec cette version.
3. **Pour chaque patient** : « Donner l'accès », puis le code montré ou
   imprimé.

Ce que les tests tiennent : le protocole et ses refus
(`patientchat::tests`), la passerelle de bout en bout avec un client
WebSocket (`passerelle::tests`), et de la passerelle à la base et retour
(`patientpost::tests`). La page elle-même a été essayée dans Firefox
contre une passerelle et un poste factices
(`passerelle::browser::serve_for_a_browser`, `--ignored`) : poignée de
main, requête scellée ouverte par le poste, réponse ouverte par la page.

## Limites connues

* **La voie directe demande un poste joignable** : une adresse IPv6
  publique que la box laisse entrer, ou une box qui ouvre le port UDP
  7748 (UPnP, PCP ou NAT-PMP, ou à la main). Une ligne derrière une traduction
  d'opérateur (CGNAT) sans IPv6 ne se joint pas ; un téléphone sur un
  réseau qui bloque l'UDP (certains Wi-Fi d'entreprise) non plus. Si
  toutes les adresses d'une officine changent entre deux visites d'un
  patient, son code ne mène plus nulle part : un nouveau code. Sans
  adresse composable, **le relais d'une officine amie du réseau** (§ 7.9
  de `SYNC.md`, en UDP) garde ce chemin sans serveur — si chaque côté
  l'a coché à part ; le code porte l'adresse du relais, que la page
  compose comme une autre.
* Sous Windows, le poste ne sait pas distinguer une adresse
  IPv6 temporaire (qui tourne en une semaine) d'une adresse stable
  (Linux et macOS les distinguent) :
  un code peut en porter une qui mourra. Les réponses rendent les
  adresses à jour à qui écrit ; un code de tableau imprimé, lui, ne les
  suit pas — le réimprimer quand l'onglet dit d'autres adresses.
* Un écran du tableau sur le réseau de l'officine joint le poste à son
  adresse locale, que « Copier le lien de l'écran » met en tête ; un
  écran ailleurs passe par l'adresse publique.
* Le tableau ne se lit que par la voie directe : rien, sur une
  passerelle, ne garantit qu'il vient de l'officine (une pharmacie de
  garde changée en chemin). Dans une conversation, il est scellé avec
  elle.
* Essayée dans Firefox ; Chrome et Safari font la même chose (c'est le
  « WebRTC direct » de libp2p), pas encore essayés ici.

* Un message n'arrive que si un poste est **en service** — ouvert,
  écran voilé ou fenêtre rangée dans la zone de notification, lancé à
  l'ouverture de session (`[bureau]`, depuis la 0.434) ; il n'y a pas de
  notification sur le téléphone du patient (une notification passerait
  par un service tiers).
* La page vit dans le navigateur du téléphone : effacer les données du
  site efface la conversation de ce téléphone (pas de la base), et il
  faut un nouveau code.
* Un poste d'une version antérieure ne voit pas les tables de la
  messagerie : mettre à jour tous les postes.
* Qui tient un code peut occuper un poste dix secondes par conversation,
  trente fois par minute et par adresse : c'est un patient de
  l'officine, dont on ferme l'accès. Une page sans code valable ne coûte
  au poste qu'une réponse « inconnu ». Une passerelle partagée par
  plusieurs officines se partage aussi ses bornes (seize attentes par
  adresse, cinq cent douze conversations à la fois, les attentes des
  postes comptées à part) : beaucoup d'adresses peuvent la saturer ; et plusieurs officines derrière une même adresse IPv4
  (traduction d'opérateur) se partagent les seize attentes — huit postes
  en tout.
* Un poste qui disparaît sans fermer sa connexion (coupure de courant)
  semble encore attendre jusqu'à cinq minutes : une page mise en relation
  avec lui échoue au bout d'une minute et réessaie.
* La vérification de la page par les postes voit une page changée pour
  tous ; GitHub pourrait en servir une autre à un seul téléphone, que
  les postes ne verraient pas. La page vient du dépôt public : ce qu'elle
  fait se lit, et chaque version en reste dans l'historique.
* Les fichiers d'un jour du journal ne sont effacés qu'au premier
  appel d'un jour nouveau, ou au lancement : une passerelle que personne
  n'appelle les garde jusque-là.
