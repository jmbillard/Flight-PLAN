#!/bin/bash
# Compila "Linhas Conectoras.plugin" (universal arm64 + x86_64) no macOS.
set -euo pipefail
cd "$(dirname "$0")"
NAME="LinhasConectoras"
BIN="linhas_conectoras"
OUT="dist"
TD="${CARGO_TARGET_DIR:-target}"

rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release -p "$BIN" --target aarch64-apple-darwin
cargo build --release -p "$BIN" --target x86_64-apple-darwin

B="$OUT/$NAME.plugin"
rm -rf "$OUT"
mkdir -p "$B/Contents/MacOS" "$B/Contents/Resources"
R="$TD/aarch64-apple-darwin/release"
cp "$R/$BIN.rsrc"        "$B/Contents/Resources/$NAME.rsrc"
printf 'eFKTFXTC' > "$B/Contents/PkgInfo"
cp mac/Info.plist "$B/Contents/Info.plist"
lipo "$TD"/{x86_64,aarch64}-apple-darwin/release/lib$BIN.dylib -create -output "$B/Contents/MacOS/$NAME"
# AE 25.2+ exige assinatura: ad-hoc serve para uso proprio
codesign --force --options runtime --timestamp=none --sign - "$B"
codesign -dv "$B" 2>&1 | head -5
lipo -info "$B/Contents/MacOS/$NAME"
nm -gU "$B/Contents/MacOS/$NAME" | grep EffectMain
( cd "$OUT" && ditto -c -k --keepParent "$NAME.plugin" "$NAME-mac.zip" )
echo "OK: $OUT/$NAME-mac.zip"
