#!/usr/bin/env bash
# Signer une version publiée, **sur ce poste**, avec la clé hors ligne de
# l'éditeur (minisign) — jamais dans la CI : une clé que la CI tient, un
# workflow compromis la tient aussi.
#
#   ./scripts/sign-release.sh v0.436.0 [clé secrète]
#
# Pour chaque binaire des postes de la version :
#   1. le fichier publié est comparé à celui que la construction a rangé
#      dans les artefacts de sa tâche (`release.yml`) — un binaire
#      remplacé dans la publication après coup ne se signe pas ;
#   2. il est signé, commentaire signé « <binaire> <version> » (ce que le
#      lanceur exige : une signature d'une autre version n'en fait pas
#      installer une autre), **pré-haché** (le lanceur refuse l'ancien
#      format) ;
#   3. la signature est relue avec la clé publique du dépôt ;
# puis les `.minisig` sont déposés. Tant qu'une version n'est pas signée,
# un lanceur qui porte la clé publique garde la version installée.
#
# Une fois, pour commencer :
#   minisign -G -p launcher/update-key.pub -s ~/.minisign/bpm-caddy.key
# puis committer `launcher/update-key.pub` **une version avant** d'en
# signer : les lanceurs la reçoivent en se remplaçant eux-mêmes. Garder la
# clé secrète hors de ce dépôt, sauvegardée.
set -euo pipefail
tag=${1:?usage: sign-release.sh vX.Y.Z [clé secrète]}
key=${2:-$HOME/.minisign/bpm-caddy.key}
root=$(cd "$(dirname "$0")/.." && pwd)
pub="$root/launcher/update-key.pub"
for tool in minisign gh sha256sum base64; do
    command -v "$tool" >/dev/null || { echo "$tool introuvable" >&2; exit 1; }
done
test -s "$key" || { echo "clé secrète introuvable : $key" >&2; exit 1; }
test -s "$pub" || { echo "pas de clé publique dans $pub" >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/publie" "$work/construit"

# La tâche qui a construit cette version, et ses artefacts.
run=$(gh run list --workflow release.yml --branch "$tag" --limit 1 \
    --json databaseId,conclusion --jq '.[] | select(.conclusion=="success") | .databaseId')
test -n "$run" || { echo "aucune construction réussie de $tag" >&2; exit 1; }
gh run download "$run" -D "$work/construit"

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
    gh release download "$tag" -p "$a" -D "$work/publie"
    built=$(find "$work/construit" -type f -name "$a" | head -1)
    test -n "$built" || { echo "$a : absent des artefacts de la construction" >&2; exit 1; }
    if [ "$(sha256sum < "$work/publie/$a")" != "$(sha256sum < "$built")" ]; then
        echo "$a : le fichier publié n'est pas celui que la construction a fait — rien n'est signé" >&2
        exit 1
    fi
    minisign -S -s "$key" -m "$work/publie/$a" -t "$a $tag" -x "$work/publie/$a.minisig"
    # Pré-haché (« ED »), le seul format que le lanceur accepte.
    alg=$(sed -n 2p "$work/publie/$a.minisig" | base64 -d 2>/dev/null | head -c 2)
    [ "$alg" = "ED" ] || { echo "$a : signature non pré-hachée ($alg) — minisign trop ancien ?" >&2; exit 1; }
    minisign -V -p "$pub" -m "$work/publie/$a" -x "$work/publie/$a.minisig" >/dev/null
done
gh release upload "$tag" "$work/publie"/*.minisig --clobber
echo "signée : $tag"
