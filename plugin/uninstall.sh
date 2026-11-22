#!/bin/sh
# Remove the plugin and native messaging host manifests installed by
# install.sh for the current user.
set -eu

dest="${XDG_DATA_HOME:-$HOME/.local/share}/pop-launcher/plugins/browser-tabs"
if [ -d "$dest" ]; then
    rm -rf "$dest"
    echo "Removed plugin from $dest"
fi

host_name="cosmic_launcher.browser_tabs"
config="${XDG_CONFIG_HOME:-$HOME/.config}"

for browser in chromium google-chrome google-chrome-beta google-chrome-unstable \
    BraveSoftware/Brave-Browser vivaldi microsoft-edge; do
    manifest="$config/$browser/NativeMessagingHosts/$host_name.json"
    [ -f "$manifest" ] || continue
    rm -f "$manifest"
    echo "Removed native host manifest $manifest"
done

echo "Restart the launcher with 'pkill pop-launcher' and remove the browser extension."
