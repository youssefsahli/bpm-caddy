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
profile=release
flag=--release
if [[ "${1:-}" == "--debug" ]]; then profile=debug; flag=; fi

cd "$root"
cargo ndk -t arm64-v8a -t x86_64 -P 26 -o "$here/app/src/main/jniLibs" \
    build -p bpm-caddy-mobile --lib $flag
# Les liaisons se lisent dans la bibliothèque construite pour l'hôte : les
# mêmes métadonnées UniFFI, sans émulateur.
cargo build -p bpm-caddy-mobile --lib $flag
cargo run -p bpm-caddy-mobile --bin uniffi-bindgen $flag -- generate \
    --library "$root/target/$profile/libbpm_caddy_mobile.so" \
    --language kotlin --no-format \
    --out-dir "$here/app/src/main/java"
echo "jniLibs et liaisons écrits dans $here/app/src/main"
