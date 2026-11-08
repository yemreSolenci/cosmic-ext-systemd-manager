# COSMIC Systemd Manager

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![COSMIC Desktop](https://img.shields.io/badge/COSMIC-Applet-blue.svg)](https://github.com/pop-os/libcosmic)
[![Rust](https://img.shields.io/badge/Rust-2021%20Edition-orange.svg)](https://www.rust-lang.org/)
[![AI Powered](https://img.shields.io/badge/AI-Powered-8A2BE2.svg)](#)

A native, lightweight, and responsive `libcosmic` panel applet for the Pop!_OS COSMIC desktop environment to monitor and control `systemd` services directly from your panel.

---

## ✨ Features

- **⚡ Instant Control**: Start, stop, and restart services directly via asynchronous D-Bus (`zbus`) with real-time state feedback.
- **🔍 Full Unit Discovery**: Discovers both running services and offline/disabled unit files on disk (e.g., custom fan control or network scripts).
- **🔄 Smart Transition Tracking**: Automatically detects `activating`, `deactivating`, and auto-restart states until the service settles.
- **📌 Pinned Services & Custom Labels**: Pin only your essential services and give them friendly aliases (e.g. `VBAN Audio` instead of `vban.service`).
- **🔎 Quick Search**: Live autocomplete search across all available services by name and description.
- **📐 Clean Layout**: Responsive layout with automatic text truncation prevents labels from overflowing panel popups.

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

### Via COSMIC Store (Flatpak)
*Once published on the official COSMIC repository:*
1. Open **COSMIC Store**.
2. Search for **Systemd Manager** and click **Install**.
3. Open **COSMIC Settings → Desktop → Panel → Add Applet** and add it to your panel.

---

### Build from Source

#### Prerequisites
```bash
# On Pop!_OS / Ubuntu
sudo apt install libwayland-dev libxkbcommon-dev libdbus-1-dev pkg-config just
```

#### Build and Install
```bash
git clone https://github.com/yemreSolenci/cosmic-systemd-manager.git
cd cosmic-systemd-manager

# Install to ~/.local (no sudo required)
just prefix=$HOME/.local install

# Reload panel
killall cosmic-panel
```
Then navigate to **COSMIC Settings → Desktop → Panel → Add Applet** and add **Systemd Manager**.

---

## ⚙️ Configuration

Pinned services are saved in standard XDG JSON format at:
```
~/.config/CosmicSystemdManager/config.json
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).

<p align="center"><sub>🤖 <i>AI-powered development with modern Rust and libcosmic.</i></sub></p>
