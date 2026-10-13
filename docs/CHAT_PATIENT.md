# Messagerie patient par QR code — options à trancher

Demande : un QR code sur la fiche du patient, qu'il scanne pour écrire à
l'officine depuis une application ou un site, côté patient.

Rien n'est construit tant que le point ci-dessous n'est pas tranché :
c'est la première fonction où une donnée de santé quitterait les postes
de l'officine pour aller chez un patient, à travers Internet.

## Ce qui change par rapport à aujourd'hui

Aujourd'hui, tout ce qui touche un patient reste dans la base chiffrée
de l'officine et sur ses postes (`replica::Flux::Dossiers`). Le compagnon
Android ne reçoit **jamais** un patient (`docs/SYNC.md` § 7.8). Le relais
(`src/relay.rs`) ne garde rien et ne voit que des octets chiffrés de bout
en bout, entre postes et téléphones de l'**équipe**.

Une messagerie patient ajoute trois choses : un appareil qui n'est pas à
l'officine (le téléphone du patient), une identité à prouver (ce patient,
et pas un autre qui a photographié le QR code), et des messages qui
attendent quand le poste est éteint.

## Le point réglementaire

Héberger des données de santé pour le compte d'un tiers demande un
hébergeur certifié **HDS** (art. L. 1111-8 du code de la santé
publique). La base de l'officine, sur ses postes, n'est pas concernée :
c'est elle qui conserve ses propres données. Un serveur qui garderait les
messages d'un patient en attendant que l'officine les lise le serait
vraisemblablement, même chiffrés de bout en bout — à faire confirmer
(CNIL, ou un juriste), pas à supposer.

## Trois chemins

**A. Sans serveur qui garde : le relais, et le poste allumé.**
Une page statique (code seulement, sans donnée) que le QR code ouvre ;
elle parle au poste de l'officine à travers le relais existant, chiffré
de bout en bout comme les postes entre eux. Le QR code porte une clé à
usage unique propre au patient. Rien n'est gardé ailleurs que dans la
base de l'officine. Limite : un message n'arrive que pendant que
l'application est ouverte à l'officine ; sinon la page le dit (« l'officine
répondra à sa réouverture ») et le garde sur le téléphone du patient.
C'est le plus proche de ce qui existe, sans hébergeur nouveau.

**B. Un hébergeur HDS.** Une boîte aux lettres chez un hébergeur
certifié, où les messages attendent, chiffrés. Messages reçus à toute
heure ; mais un contrat, un coût, et une dépendance hors de l'officine.

**C. La messagerie de Mon espace santé (MSSanté).** Le canal officiel
entre professionnels et patients : l'officine écrit au patient depuis une
messagerie MSSanté, le patient répond depuis Mon espace santé. Pas
d'hébergement à notre charge ; mais une intégration MSSanté (opérateur
référencé, carte CPS ou e-CPS), et le QR code mènerait vers Mon espace
santé plutôt que vers une page à nous.

## Ce qu'il faut décider

1. Le chemin : A, B ou C.
2. Le patient n'écrit-il qu'après un geste de l'officine (le QR code
   imprimé sur un document qu'on lui remet), ou à tout moment ?
3. Les pièces jointes (photo d'une ordonnance, d'une boîte) : oui ou non
   — une photo d'ordonnance est une donnée de santé de plus.

Une fois tranché, le QR code se pose sur les documents imprimés (fiche de
traitement, feuille de l'infirmier(ère), carnet) par le même chemin que
les autres marqueurs de `pdf::DOCS`.
