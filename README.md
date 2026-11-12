# Systemd Manager for the COSMIC™ desktop

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![COSMIC™ Desktop](https://img.shields.io/badge/COSMIC™-Applet-blue.svg)](https://github.com/pop-os/libcosmic)
[![Rust](https://img.shields.io/badge/Rust-2021%20Edition-orange.svg)](https://www.rust-lang.org/)

A native, lightweight, and responsive `libcosmic` panel applet for the COSMIC™ desktop environment to monitor and control `systemd` services directly from your panel.

<p align="center">
  <img src="assets/demo.gif" alt="Systemd Manager Demo" width="500" />
</p>

---

## ✨ Features

- **⚡ Instant Control**: Start, stop, and restart services directly via asynchronous D-Bus (`zbus`) with real-time state feedback.
- **🔍 Full Unit Discovery**: Discovers both running services and offline/disabled unit files on disk (e.g., custom fan control or network scripts).
- **🔄 Smart Transition Tracking**: Automatically detects `activating`, `deactivating`, and auto-restart states until the service settles.
- **📌 Pinned Services & Custom Labels**: Pin only your essential services and give them friendly aliases (e.g. `VBAN Audio` instead of `vban.service`).
- **🔎 Quick Search**: Live autocomplete search across all available services by name and description.
- **📐 Clean Layout**: Responsive layout with automatic text truncation prevents labels from overflowing panel popups.

<p align="center">
  <img src="assets/demo_service_add.gif" alt="Searching and Pinning Services Demo" width="450" />
  <br>
  <em>Live search autocomplete and 1-click service pinning.</em>
</p>

---

## 🔐 Permissions & Authentication (Polkit)

Linux requires administrative privileges to start or stop **system-wide** background services (such as Docker, VPNs, or hardware fan controllers). Systemd uses **PolicyKit (Polkit)** to verify whether a user is allowed to manage these services.

### Option 1: 1-Click Instant Toggling ⭐ (Recommended)

To make the panel applet feel truly seamless—toggling services instantly with a single click instead of interrupting you with a password prompt every time—create a Polkit rule:

1. Create the rule file:
```bash
sudo nano /etc/polkit-1/rules.d/49-cosmic-systemd.rules
```

2. Paste the following configuration (applies to all users in the `sudo` group):
```javascript
polkit.addRule(function(action, subject) {
    if (action.id == "org.freedesktop.systemd1.manage-units" &&
        subject.isInGroup("sudo")) {
        return polkit.Result.YES;
    }
});
```

3. Restart Polkit to apply:
```bash
sudo systemctl restart polkit
```

> **Tip (Strict Security):** If you prefer to allow only specific units, replace `return polkit.Result.YES;` with an allowlist check:
> `var allowed = ["docker.service", "vban.service"]; if (allowed.indexOf(action.lookup("unit")) !== -1) return polkit.Result.YES;`

---

### Option 2: Default Prompt (No Setup)

If you do not configure any Polkit rules, the applet still works out of the box. Whenever you flip a service toggle, COSMIC's native authentication window will pop up asking for your password.

---

## 🚀 Installation

### Option 1: Via COSMIC App Store (Flatpak)
*Available once merged into the official COSMIC Flatpak repository:*
1. Open **COSMIC App Store**.
2. Search for **Systemd Manager** and click **Install**.
3. Open **COSMIC Settings → Desktop → Panel → Add Applet** and add it to your panel.

---

### Option 2: Precompiled Binary (Quick Install)
Download the latest `cosmic-ext-applet-systemd-manager-v0.1.0-x86_64-linux.tar.gz` from the [Releases](https://github.com/yemreSolenci/cosmic-ext-systemd-manager/releases) page:
```bash
tar -xzf cosmic-ext-applet-systemd-manager-v0.1.0-x86_64-linux.tar.gz
cd cosmic-ext-applet-systemd-manager-v0.1.0-x86_64-linux
./install.sh
killall cosmic-panel
```

---

### Option 3: Build from Source

#### Prerequisites
```bash
# On Pop!_OS / Ubuntu
sudo apt install libwayland-dev libxkbcommon-dev libdbus-1-dev pkg-config
```

#### Build and Install
```bash
git clone https://github.com/yemreSolenci/cosmic-ext-systemd-manager.git
cd cosmic-ext-systemd-manager

cargo build --release
mkdir -p ~/.local/bin ~/.local/share/applications ~/.local/share/metainfo ~/.local/share/icons/hicolor/scalable/apps
cp target/release/cosmic-ext-applet-systemd-manager ~/.local/bin/
cp res/io.github.yemresolenci.CosmicExtAppletSystemdManager.desktop ~/.local/share/applications/
cp res/io.github.yemresolenci.CosmicExtAppletSystemdManager.metainfo.xml ~/.local/share/metainfo/
cp res/icons/hicolor/scalable/apps/io.github.yemresolenci.CosmicExtAppletSystemdManager.svg ~/.local/share/icons/hicolor/scalable/apps/

# Reload panel
killall cosmic-panel
```
*(Tip: If you have `just` installed, you can simply run `just prefix=$HOME/.local install`)*
Then navigate to **COSMIC Settings → Desktop → Panel → Add Applet** and add **Systemd Manager**.

---

## ⚙️ Configuration

Pinned services are saved in standard XDG JSON format at:
```
~/.config/CosmicExtAppletSystemdManager/config.json
```

---

## 📄 License & Trademark Notice

This project is licensed under the [MIT License](LICENSE).

*COSMIC™ is a trademark of System76, Inc. This project is an independent community project and is not affiliated with, endorsed by, or sponsored by System76.*

<p align="center"><sub>🤖 <i>Developed with AI assistance using modern Rust and libcosmic.</i></sub></p>
