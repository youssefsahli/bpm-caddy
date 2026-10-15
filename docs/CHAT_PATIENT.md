# Messagerie patient par QR code

Demande : un QR code sur la fiche du patient, qu'il scanne pour écrire à
l'officine depuis une application ou un site, côté patient.

**Décidé le 14/10/2026 : le chemin A** — pas de serveur qui garde, le
poste allumé. Ce document dit ce qui a été construit, ce que cela
demande, et ce qui reste ouvert.

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
`https://bpmcaddy.github.io/<empreinte>/#b=<boîte>&k=<clé>&p=<passerelle>`.
Le fragment (après `#`) n'est envoyé à personne par le navigateur : la
clé ne quitte pas le téléphone, et GitHub ne sait pas quelle passerelle
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

**La passerelle** (`src/passerelle.rs`, binaire `bpm-passerelle`).
Pourquoi il en faut une : un navigateur ne parle pas TCP. Depuis une
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
dossier). Chaque poste de bureau ouvert attend à la passerelle (deux
attentes à la fois) et répond avec la base. La clé et les messages
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
  adresse, cinq cent douze connexions) : beaucoup d'adresses peuvent la
  saturer ; et plusieurs officines derrière une même adresse IPv4
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
