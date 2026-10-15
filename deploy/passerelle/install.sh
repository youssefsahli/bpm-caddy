#!/usr/bin/env bash
# Installer la passerelle sur un serveur Debian/Ubuntu (systemd + Caddy).
#
#   sudo ./install.sh messagerie.mondomaine.fr chemin/vers/bpm-passerelle
#
# Le binaire se construit depuis le dépôt, sur n'importe quelle machine
# Linux x86-64 :
#   cargo build --release --bin bpm-passerelle --no-default-features --features sync
#   → target/release/bpm-passerelle
#
# Ce que fait ce script : copie le binaire dans /usr/local/bin, pose le
# service systemd (utilisateur jetable, journal des connexions gardé 92
# jours), installe Caddy s'il manque et lui donne le nom de domaine
# (certificat Let's Encrypt automatique), ouvre 80/443 et 7747 si ufw est
# là, puis démarre le tout. Le nom de domaine doit déjà pointer sur ce
# serveur (enregistrement A et/ou AAAA).
set -euo pipefail
domain=${1:?usage: install.sh nom.de.domaine chemin/du/binaire}
bin=${2:?usage: install.sh nom.de.domaine chemin/du/binaire}
here=$(cd "$(dirname "$0")" && pwd)
[ "$(id -u)" = 0 ] || { echo "à lancer en root (sudo)" >&2; exit 1; }
case "$domain" in
    *[!a-z0-9.-]* | "" | .* | *.) echo "nom de domaine illisible : $domain" >&2; exit 1 ;;
esac
head -c 4 "$bin" | grep -q $'\x7fELF' || { echo "$bin n'est pas un exécutable Linux" >&2; exit 1; }

install -m 0755 "$bin" /usr/local/bin/bpm-passerelle
install -m 0644 "$here/bpm-passerelle.service" /etc/systemd/system/bpm-passerelle.service

if ! command -v caddy >/dev/null; then
    apt-get update
    apt-get install -y caddy
fi
mkdir -p /etc/caddy/sites
sed "s/messagerie\.example\.fr/$domain/" "$here/Caddyfile" > /etc/caddy/sites/bpm-passerelle.caddy
grep -q 'import sites/\*' /etc/caddy/Caddyfile 2>/dev/null \
    || echo 'import sites/*' >> /etc/caddy/Caddyfile

if command -v ufw >/dev/null && ufw status | grep -q active; then
    ufw allow 80/tcp
    ufw allow 443/tcp
    ufw allow 7747/tcp
fi

systemctl daemon-reload
systemctl enable --now bpm-passerelle
systemctl reload caddy || systemctl restart caddy
sleep 2
systemctl --no-pager --lines=3 status bpm-passerelle || true
echo
echo "Passerelle en service : https://$domain (pages), port 7747 (postes)."
echo "Pour la livrer avec BPM-Caddy : variable BPM_CADDY_PASSERELLE = $domain"
echo "dans les réglages du dépôt (Settings › Secrets and variables › Actions › Variables),"
echo "puis une version publiée. Voir deploy/passerelle/README.md."
