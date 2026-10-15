#!/usr/bin/env bash
# Signer une version publiée, **sur ce poste**, avec la clé hors ligne de
# l'éditeur (minisign) — jamais dans la CI : une clé que la CI tient, un
# workflow compromis la tient aussi.
#
#   ./scripts/sign-release.sh v0.436.0 [clé secrète]
#
# Télécharge les binaires des postes de la version, signe chacun avec un
# commentaire signé « <binaire> <version> » (ce que le lanceur exige :
# une signature d'une autre version n'en fait pas installer une autre),
# et dépose les `.minisig` à côté. Tant qu'une version n'est pas signée,
# un lanceur qui porte la clé publique garde la version installée.
#
# Une fois, pour commencer :
#   minisign -G -p launcher/update-key.pub -s ~/.minisign/bpm-caddy.key
# puis committer `launcher/update-key.pub` : les lanceurs construits
# ensuite vérifient. Garder la clé secrète hors de ce dépôt, sauvegardée.
set -euo pipefail
tag=${1:?usage: sign-release.sh vX.Y.Z [clé secrète]}
key=${2:-$HOME/.minisign/bpm-caddy.key}
command -v minisign >/dev/null || { echo "minisign introuvable" >&2; exit 1; }
command -v gh >/dev/null || { echo "gh introuvable" >&2; exit 1; }
test -s "$key" || { echo "clé secrète introuvable : $key" >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
assets=(
    bpm-caddy-linux-x86_64
    bpm-caddy-windows-x86_64.exe
    bpm-caddy-macos-arm64
    bpm-caddy-launcher-linux-x86_64
    bpm-caddy-launcher-windows-x86_64.exe
    bpm-caddy-launcher-macos-arm64
    bpm-caddy-setup-windows-x86_64.exe
)
for a in "${assets[@]}"; do
    gh release download "$tag" -p "$a" -D "$work"
    minisign -S -s "$key" -m "$work/$a" -t "$a $tag" -x "$work/$a.minisig"
    # Relue avec la clé publique du dépôt, avant de déposer.
    minisign -V -p launcher/update-key.pub -m "$work/$a" -x "$work/$a.minisig" >/dev/null
done
gh release upload "$tag" "$work"/*.minisig --clobber
echo "signée : $tag"
