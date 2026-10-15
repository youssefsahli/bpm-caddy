# Tenir la passerelle des patients

La passerelle relie la page du patient (publiée sur
https://bpmcaddy.github.io/) au poste de l'officine qui l'attend. Elle ne
garde aucun message et n'en lit aucun ; elle tient un journal des
connexions (l'heure, l'adresse, la boîte, l'issue), gardé 92 jours. Qui
la tient pour des officines est leur **sous-traitant** (RGPD, art. 28) :
voir `docs/CHAT_PATIENT.md` et l'avis de l'avocat.

## Ce qu'il faut

- Un petit serveur Linux (Debian 12, Ubuntu 22.04 ou plus récents),
  joignable sur Internet : un vCPU et 512 Mo suffisent.
- Un nom de domaine qui pointe dessus (enregistrement A, et AAAA si le
  serveur a une adresse IPv6), par exemple `messagerie.mondomaine.fr`.
- Les ports 80 et 443 (le certificat, les pages) et **7747** (les postes)
  ouverts vers Internet.

## Installer

Sur une machine qui a le dépôt et Rust :

```sh
cargo build --release --bin bpm-passerelle --no-default-features --features sync
scp target/release/bpm-passerelle deploy/passerelle/* serveur:/tmp/passerelle/
```

Sur le serveur :

```sh
cd /tmp/passerelle && sudo ./install.sh messagerie.mondomaine.fr ./bpm-passerelle
```

Le script pose le binaire dans `/usr/local/bin`, le service
`bpm-passerelle` (un utilisateur jetable de systemd, sans droits, qui
n'écrit que `/var/log/bpm-passerelle`), installe Caddy s'il manque et lui
donne le nom (certificat Let's Encrypt de lui-même), ouvre les ports si
`ufw` est actif, et démarre le tout.

## La livrer avec BPM-Caddy

Dans les réglages du dépôt : *Settings › Secrets and variables › Actions
› Variables*, `BPM_CADDY_PASSERELLE` = `messagerie.mondomaine.fr`. Les
versions publiées ensuite l'écrivent : la messagerie est prête à
l'installation, sans réglage dans l'officine.

**Avant** : le contrat de sous-traitance et les conditions, relus par
l'avocat, dans `assets/passerelle-conditions.md`. Livrés avec la même
version, chaque officine les accepte une fois avant que la passerelle la
serve (le poste, l'heure, la version du texte sont gardés).

## La suivre

```sh
systemctl status bpm-passerelle
journalctl -u bpm-passerelle -f          # ce qu'elle dit d'elle-même
ls /var/log/bpm-passerelle/              # un fichier de connexions par jour
```

Mettre à jour : reconstruire, puis relancer `install.sh` avec le nouveau
binaire (il le remplace et relance le service) — les postes se
reconnectent d'eux-mêmes, les pages ouvertes réessaient.

Bornes d'une passerelle (voir `src/passerelle.rs`) : 1 024 boîtes
(officines), quatre attentes par boîte, seize par adresse, et 512
conversations à la fois (pages en cours, postes qui se présentent) ; les
attentes ne comptent pas dans les conversations. Au-delà de mille
officines, une seconde passerelle sous un autre nom. Sans IPv6 sur le
serveur, `install.sh` n'ouvre le port des postes qu'en IPv4.
