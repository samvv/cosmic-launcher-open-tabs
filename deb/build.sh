#!/bin/sh
# Build the Debian package. Needs Rust and cargo-deb (`cargo install cargo-deb`).
# Extra arguments are passed to cargo-deb, e.g. --target aarch64-unknown-linux-gnu.
# The .deb ends up in plugin/target/debian/.
set -eu
cd "$(dirname "$0")/../plugin"

out=target/deb-assets
mkdir -p "$out"
sh native-host-manifest.sh /usr/lib/pop-launcher/plugins/browser-tabs/browser-tabs \
    > "$out/host.json"

cargo deb --locked "$@"
