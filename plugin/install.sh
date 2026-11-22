#!/bin/sh
# Build the plugin and install it for the current user, along with the native
# messaging host manifest for every Chromium-based browser that is present.
set -eu
cd "$(dirname "$0")"
cargo build --release

dest="${XDG_DATA_HOME:-$HOME/.local/share}/pop-launcher/plugins/browser-tabs"
mkdir -p "$dest"
install -m 644 plugin.ron "$dest/plugin.ron"
install -m 755 target/release/browser-tabs "$dest/browser-tabs"
echo "Installed plugin to $dest"

# Extensions allowed to start the host: the dev build (ID fixed by the "key" in
# chromium/manifest.json). Add the Chrome Web Store ID here once it's assigned.
extension_ids="hkhjldjlbnpjjohigakmhnppinkbaaad"
origins=$(printf '"chrome-extension://%s/", ' $extension_ids)
origins="[${origins%, }]"
host_name="cosmic_launcher.browser_tabs"
config="${XDG_CONFIG_HOME:-$HOME/.config}"

for browser in chromium google-chrome google-chrome-beta google-chrome-unstable \
    BraveSoftware/Brave-Browser vivaldi microsoft-edge; do
    [ -d "$config/$browser" ] || continue
    hosts="$config/$browser/NativeMessagingHosts"
    mkdir -p "$hosts"
    cat > "$hosts/$host_name.json" <<JSON
{
  "name": "$host_name",
  "description": "Browser Tabs for COSMIC Launcher",
  "path": "$dest/browser-tabs",
  "type": "stdio",
  "allowed_origins": $origins
}
JSON
    echo "Installed native host manifest to $hosts"
done
