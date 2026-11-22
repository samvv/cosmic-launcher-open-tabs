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

host_name="be.samvervaeck.cosmic_launcher_open_tabs"
config="${XDG_CONFIG_HOME:-$HOME/.config}"

for browser in chromium google-chrome google-chrome-beta google-chrome-unstable \
    BraveSoftware/Brave-Browser vivaldi microsoft-edge; do
    [ -d "$config/$browser" ] || continue
    hosts="$config/$browser/NativeMessagingHosts"
    mkdir -p "$hosts"
    ./native-host-manifest.sh "$dest/browser-tabs" > "$hosts/$host_name.json"
    echo "Installed native host manifest to $hosts"
done
