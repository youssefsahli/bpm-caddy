#!/usr/bin/env bash
# Construit la bibliothèque Rust du compagnon pour Android et écrit ses
# liaisons Kotlin. À lancer avant Gradle (`./gradlew assembleDebug`).
#
#   ./build-rust.sh            # arm64 + x86_64 (émulateur), en release
#   ./build-rust.sh --debug    # plus rapide, pour essayer
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/.." && pwd)"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.2.12479018}"
flag=--release
if [[ "${1:-}" == "--debug" ]]; then flag=; fi

cd "$root"
cargo ndk -t arm64-v8a -t x86_64 -P 26 -o "$here/app/src/main/jniLibs" \
    build -p bpm-caddy-mobile --lib $flag
# Les liaisons se lisent dans la bibliothèque construite pour l'hôte : les
# mêmes métadonnées UniFFI, sans émulateur. **Toujours la version de
# débogage** : le profil release de l'atelier retire les symboles
# (`strip = true`), métadonnées comprises, et le générateur n'écrit alors
# rien — sans le dire.
cargo build -p bpm-caddy-mobile --lib
out="$here/app/src/main/java/uniffi/bpm_caddy_mobile/bpm_caddy_mobile.kt"
rm -f "$out"
cargo run -p bpm-caddy-mobile --bin uniffi-bindgen -- generate \
    --library "$root/target/debug/libbpm_caddy_mobile.so" \
    --language kotlin --no-format \
    --out-dir "$here/app/src/main/java"
test -s "$out" || { echo "liaisons Kotlin non écrites" >&2; exit 1; }
echo "jniLibs et liaisons écrits dans $here/app/src/main"
