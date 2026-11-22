#!/bin/sh
# Print the native messaging host manifest for the given host binary path.
# Used by install.sh and the AUR package so both allow the same extensions.
set -eu
[ $# -eq 1 ] || { echo "usage: $0 <absolute path to browser-tabs>" >&2; exit 1; }

# Extensions allowed to start the host: the dev build (ID fixed by the "key" in
# chromium/manifest.json). Add the Chrome Web Store ID here once it's assigned.
extension_ids="hkhjldjlbnpjjohigakmhnppinkbaaad"

origins=$(printf '"chrome-extension://%s/", ' $extension_ids)
cat <<JSON
{
  "name": "cosmic_launcher.browser_tabs",
  "description": "Browser Tabs for COSMIC Launcher",
  "path": "$1",
  "type": "stdio",
  "allowed_origins": [${origins%, }]
}
JSON
