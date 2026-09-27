#!/usr/bin/env bash
# Vérifie qu'un APK est signé **de la clé d'importation de l'officine** —
# pas seulement signé : un APK d'une autre clé ne s'installe pas par-dessus
# celui des téléphones, et le publier serait une impasse pour l'équipe.
#
#   ./android/verify-apk.sh chemin/vers/app-release.apk
set -euo pipefail
apk=${1:?chemin du fichier APK}
# L'empreinte SHA-256 du certificat de la clé d'importation (docs/ANDROID.md).
expected=7e75f29a4c82e9d2d0ffd09137ad106a9ba7dff1ea645a4a41d4c26291bf681a

if [[ ! -f "$apk" ]]; then
    echo "APK absent : $apk" >&2
    ls -l "$(dirname "$apk")" >&2 || true
    exit 1
fi
sdk=${ANDROID_HOME:-$HOME/Android/Sdk}
signer=$(ls -d "$sdk"/build-tools/*/ | sort -V | tail -1)apksigner
echo "apksigner : $signer"
out=$("$signer" verify --print-certs "$apk" 2>&1) || {
    echo "$out" >&2
    echo "signature invalide" >&2
    exit 1
}
echo "$out" | grep -v "^WARNING"
got=$(echo "$out" | sed -n 's/.*certificate SHA-256 digest: *//p' | head -1)
if [[ "$got" != "$expected" ]]; then
    echo "signé d'une autre clé : $got (attendu $expected)" >&2
    exit 1
fi
echo "signé de la clé d'importation ($expected)"
