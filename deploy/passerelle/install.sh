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
# Sans IPv6 sur ce serveur (ipv6.disable=1), « [::] » ne s'ouvre pas et le
# service redémarrerait sans fin : les postes en IPv4 seulement.
if [ ! -e /proc/net/if_inet6 ]; then
    sed -i 's/--postes \[::\]:7747/--postes 0.0.0.0:7747/' /etc/systemd/system/bpm-passerelle.service
fi

if ! command -v caddy >/dev/null; then
    apt-get update
    # Caddy est dans Debian 12 et Ubuntu 23.04 et suivantes ; ailleurs, son
    # dépôt officiel.
    if ! apt-cache policy caddy | grep -q 'Candidate: [0-9]'; then
        apt-get install -y debian-keyring debian-archive-keyring apt-transport-https curl gnupg
        curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' \
            | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
        curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' \
            > /etc/apt/sources.list.d/caddy-stable.list
        apt-get update
    fi
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
systemctl enable bpm-passerelle
# Relancé, pas seulement allumé : réinstaller met à jour.
systemctl restart bpm-passerelle
systemctl reload caddy || systemctl restart caddy
sleep 2
systemctl --no-pager --lines=3 status bpm-passerelle || true
echo
echo "Passerelle en service : https://$domain (pages), port 7747 (postes)."
echo "Pour la livrer avec BPM-Caddy : variable BPM_CADDY_PASSERELLE = $domain"
echo "dans les réglages du dépôt (Settings › Secrets and variables › Actions › Variables),"
echo "puis une version publiée. Voir deploy/passerelle/README.md."
