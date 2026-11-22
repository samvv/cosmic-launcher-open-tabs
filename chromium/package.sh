#!/bin/sh
# Build the zip to upload to the Chrome Web Store.
#
# The Web Store rejects the "key" field (it assigns the ID itself), so it is
# stripped here. Keep it in manifest.json for development: once the store has
# assigned an ID, replace it with the store's public key (Developer Dashboard →
# Package → View public key) so unpacked builds share the store's ID.
set -eu
cd "$(dirname "$0")"

version=$(jq -r .version manifest.json)
out="$PWD/dist/browser-tabs-$version.zip"
staging=$(mktemp -d)
trap 'rm -rf "$staging"' EXIT

# Re-render icons from the SVG source.
for size in 16 48 128; do
    rsvg-convert -w "$size" -h "$size" icons/icon.svg -o "icons/icon$size.png"
done

cp background.js welcome.html welcome.js welcome.css "$staging/"
mkdir "$staging/icons"
cp icons/icon16.png icons/icon48.png icons/icon128.png "$staging/icons/"
jq 'del(.key)' manifest.json > "$staging/manifest.json"

mkdir -p dist
rm -f "$out"
(cd "$staging" && zip -qr -X "$out" .)
echo "Wrote $out"
