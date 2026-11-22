# COSMIC Launcher Open Tabs

Search your open browser tabs from COSMIC Launcher and switch to them.

Works with Chromium-based browsers. It has two parts: a launcher plugin and a
browser extension. The extension sends your tabs to the plugin over a local
socket; nothing leaves your computer.

## Install

**1. Launcher plugin**

On Arch Linux, from the AUR:

```sh
paru -S cosmic-launcher-open-tabs-git
```

On Pop!_OS, Ubuntu or Debian, download the `.deb` from the
[latest release](https://github.com/samvv/cosmic-launcher-open-tabs/releases/latest)
and install it with `sudo apt install ./cosmic-launcher-open-tabs_*.deb`. Or
build it yourself (needs Rust and `cargo install cargo-deb`):

```sh
deb/build.sh
sudo apt install ./plugin/target/debian/cosmic-launcher-open-tabs_*.deb
```

From source (needs Rust), for the current user:

```sh
plugin/install.sh
```

**2. Browser extension**

Open `chrome://extensions`, turn on Developer mode, click **Load unpacked**
and select the `chromium/` folder.

Then restart the launcher service with `pkill pop-launcher`.

## Uninstall

Run `plugin/uninstall.sh` (or `paru -R cosmic-launcher-open-tabs-git` for the
AUR package, `sudo apt remove cosmic-launcher-open-tabs` for the `.deb`), remove the extension from `chrome://extensions`, then run
`pkill pop-launcher`.

## Usage

Type part of a tab's title or address in COSMIC Launcher and pick a result to
switch to that tab. Prefix your search with `tab ` to show only tabs.

## Limitations

- Flatpak and Snap builds of the browser can't talk to the plugin.
- The AUR and Debian packages register the plugin with Chromium, Chrome and
  Edge. For
  other browsers, run `plugin/install.sh`.

## License

[MPL-2.0](LICENSE)
