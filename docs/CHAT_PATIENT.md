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
`https://<passerelle>/#b=<boîte>&k=<clé>`. Le fragment (après `#`) n'est
envoyé à personne par le navigateur : la clé ne quitte pas le téléphone.
La page (`assets/patient/`) est du HTML, du CSS et un script sans
bibliothèque ; elle chiffre avec WebCrypto. Une fois la clé gardée sur
le téléphone (`localStorage`), elle la retire de la barre d'adresse.
Elle dit qu'elle n'est pas un service d'urgence (15, 112). Avant tout
envoi, elle vérifie que le navigateur chiffre comme le poste (des
valeurs connues, que `patientchat::tests` tient des deux côtés).

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
qui porte le certificat ; elle sert la page, tient les postes qui
attendent, et passe les trames de l'un à l'autre sans les lire. Elle ne
garde rien : sans poste qui attend, la page l'apprend (fermeture 4004)
et garde le message sur le téléphone.

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

## Le point réglementaire, toujours à faire confirmer

Héberger des données de santé pour le compte d'un tiers demande un
hébergeur certifié **HDS** (art. L. 1111-8 du code de la santé
publique). Le chemin A a été choisi parce qu'il **n'héberge rien** : la
passerelle ne fait que passer des octets chiffrés de bout en bout, sans
la clé, et n'écrit rien sur un disque. La base de l'officine, sur ses
postes, reste là où elle était. Que ce transit sans stockage échappe à
l'obligation est la lecture la plus probable — **à faire confirmer**
(CNIL, ou un juriste), pas à supposer. Ce que la passerelle sait :
qu'une boîte échange, quand, combien, depuis quelle adresse IP.

## Mettre en service

1. **Une passerelle**, une fois pour toutes les officines qui s'en
   servent : un petit serveur, un nom de domaine, et un proxy qui porte
   le certificat. Avec Caddy :

   ```text
   messagerie.example.fr {
       reverse_proxy 127.0.0.1:8746
   }
   ```

   (avec nginx, garder l'en-tête `Host` : `proxy_set_header Host $host`,
   et passer la mise à niveau WebSocket), puis `bpm-passerelle` (construit par
   `cargo build --release --bin bpm-passerelle --no-default-features --features sync`),
   qui sert la page sur `127.0.0.1:8746` et attend les postes sur le port
   **7747**, à ouvrir vers Internet. Qui la tient pourrait servir une page
   modifiée qui lirait les messages : c'est l'éditeur, ou une officine de
   confiance, qui la tient.
2. **Dans l'officine**, une fois : onglet « Messagerie » d'un dossier,
   « Régler la passerelle… », son nom de domaine. Le poste tire le secret
   de la boîte de l'officine ; le réglage vaut pour tous ses postes.
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

* Un message n'arrive que si un poste est ouvert ; il n'y a pas de
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
