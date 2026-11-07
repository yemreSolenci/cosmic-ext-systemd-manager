# COSMIC Systemd Manager

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![COSMIC Desktop](https://img.shields.io/badge/COSMIC-Applet-blue.svg)](https://github.com/pop-os/libcosmic)
[![Rust](https://img.shields.io/badge/Rust-2021%20Edition-orange.svg)](https://www.rust-lang.org/)

A native, lightweight, and responsive `libcosmic` panel applet for the Pop!_OS COSMIC desktop environment that allows you to monitor and control your favorite `systemd` services directly from your panel.

Inspired by popular GNOME extensions, this applet provides a deeply integrated, lightning-fast user experience for managing background daemons without opening a terminal.

---

## ✨ Features

- **⚡ Real-Time D-Bus Integration**: Directly communicates with `org.freedesktop.systemd1` over asynchronous D-Bus (`zbus`) without invoking external shell binaries or polling `systemctl` CLI tools.
- **🔍 Full Unit Discovery (Loaded & Unloaded)**: Discovers both active/running services in memory and disabled/inactive `.service` unit files residing on disk (`/etc/systemd/system`, `/usr/lib/systemd/system`, etc.) — perfect for custom services like fan controllers (`fan100.service`, `nbfc.service`) or network daemons.
- **🔄 Smart Transition & Auto-Restart Tracking**: Detects transitional states (`activating`, `deactivating`, `reloading`, `auto-restart`) and automatically polls until the service settles into its final operational state.
- **📌 Persistent Pinned Services**: Keep your panel clean. Pin only the services you care about (e.g., `docker.service`, `vban.service`, `zapret.service`). Pinned services survive reboots and are saved to standard XDG user configuration.
- **🏷️ Custom Aliases & Labels**: Rename cryptic system unit IDs into clean, friendly labels (e.g., rename `vban.service` to `VBAN Audio`).
- **🔎 Relevance-Ranked Autocomplete Search**: Quickly find any installed system service. Searches both unit file names and embedded `Description=` fields, prioritizing prefix matches and excluding already pinned units.
- **📐 Overflow-Proof Layout**: Single-line text truncation and intelligent vertical column alignment prevent long service names or statuses from overlapping toggles and buttons.
- **🔒 Security-Hardened**: Built-in unit name validation prevents path traversal or unintended target activation (`.target` / `.mount`), enforces bounded file reads, and saves configuration with restrictive owner-only permissions (`0600`).

---

## 🚀 Installation

### Option 1: Install via COSMIC Store (Flatpak)

*Once published on the official COSMIC Flatpak repository:*
1. Open **COSMIC Store**.
2. Search for **Systemd Manager**.
3. Click **Install**.
4. Open **COSMIC Settings → Desktop → Panel → Add Applet** and add **Systemd Manager** to your top bar or dock.

---

### Option 2: Build from Source

#### Prerequisites
Ensure you have the Rust toolchain and development headers installed. On Pop!_OS / Ubuntu:

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# System libraries and build utilities
sudo apt install libwayland-dev libxkbcommon-dev libdbus-1-dev pkg-config just
```

#### 1. Clone the Repository
```bash
git clone https://github.com/yemreSolenci/cosmic-systemd-manager.git
cd cosmic-systemd-manager
```

#### 2. Build and Install
Using `just` (recommended for standard local installation):

```bash
# Install to ~/.local (no root required)
just prefix=$HOME/.local install
```

*Or manually build with Cargo:*
```bash
cargo build --release
mkdir -p ~/.local/bin ~/.local/share/applications ~/.local/share/metainfo ~/.local/share/icons/hicolor/scalable/apps

cp target/release/cosmic-systemd-applet ~/.local/bin/
cp res/io.github.yemresolenci.CosmicSystemdManager.desktop ~/.local/share/applications/
cp res/io.github.yemresolenci.CosmicSystemdManager.metainfo.xml ~/.local/share/metainfo/
cp res/icons/hicolor/scalable/apps/io.github.yemresolenci.CosmicSystemdManager.svg ~/.local/share/icons/hicolor/scalable/apps/
```

Ensure `~/.local/bin` is in your `$PATH` (e.g., in `~/.bashrc` or `~/.profile`).

#### 3. Register with COSMIC Panel
Restart `cosmic-panel` to load newly registered applets:
```bash
killall cosmic-panel
```

Then navigate to **COSMIC Settings → Desktop → Panel → Add Applet** and add **Systemd Manager**.

---

## 🔐 Polkit Authentication & Permissions

When managing **system-wide** services (like `docker.service` or hardware controllers), systemd delegates authorization to **PolicyKit (Polkit)**.

### 1. Default Behavior (Zero Configuration Required)
Out of the box, **no configuration is required**:
- When you click a toggle to start, stop, or restart a system service, COSMIC's native **Polkit Authentication Agent** automatically opens a graphical password dialog.
- Enter your user password, and the service transitions immediately.

---

### 2. Optional: 1-Click Passwordless Toggling (For Power Users)
If you prefer toggles to flip instantaneously without typing your password each time, configure a single Polkit rule.

Create the rule file:
```bash
sudo nano /etc/polkit-1/rules.d/49-cosmic-systemd.rules
```

#### Method A: Allow All Services for Admin Users (Easiest)
Paste the following rule:
```javascript
polkit.addRule(function(action, subject) {
    if (action.id == "org.freedesktop.systemd1.manage-units" &&
        subject.isInGroup("sudo")) {
        return polkit.Result.YES;
    }
});
```

#### Method B: Least-Privilege Allowlist (Recommended for Security)
Restrict passwordless toggling strictly to a specific list of services you frequently manage:
```javascript
polkit.addRule(function(action, subject) {
    var allowedUnits = [
        "docker.service",
        "vban.service",
        "fan100.service",
        "zapret.service"
    ];
    if (action.id == "org.freedesktop.systemd1.manage-units" &&
        subject.isInGroup("sudo") &&
        allowedUnits.indexOf(action.lookup("unit")) !== -1) {
        return polkit.Result.YES;
    }
});
```

Apply the rule by restarting the Polkit daemon:
```bash
sudo systemctl restart polkit
```

---

## ⚙️ Configuration

Pinned services and their labels are automatically persisted according to the XDG specification:

```
~/.config/CosmicSystemdManager/config.json
```

The configuration is saved as clean, human-readable JSON:
```json
[
  {
    "id": "docker.service",
    "label": "Docker Daemon"
  },
  {
    "id": "vban.service",
    "label": "VBAN Audio Stream"
  },
  {
    "id": "fan100.service",
    "label": "Max Fan Speed"
  }
]
```

*(Note: If migrating from older versions, existing configuration files under legacy paths are automatically imported on first launch).*

---

## 🛠 Architecture & Tech Stack

| Component | Technology | Purpose |
|---|---|---|
| **GUI Framework** | [`libcosmic`](https://github.com/pop-os/libcosmic) | Declarative UI, Wayland layer-shell protocol, native COSMIC widgets |
| **D-Bus Engine** | [`zbus`](https://gitlab.freedesktop.org/dbus/zbus) | Non-blocking async communication with `org.freedesktop.systemd1` |
| **Async Runtime** | [`tokio`](https://tokio.rs/) | Multi-threaded async task scheduling and transition timers |
| **Serialization** | [`serde`](https://serde.rs/) & `serde_json` | Type-safe JSON serialization for pinned service configurations |
| **XDG Directories**| [`directories`](https://crates.io/crates/directories) | Standardized XDG base directory resolution |

### Source Layout

```
src/
├── main.rs      # Applet entry point (cosmic::applet::run)
├── app.rs       # Application state machine, message handling, background subscription
├── ui.rs        # Declarative views: Main popup, Manage, Edit label, Confirm add
├── systemd.rs   # Type-safe D-Bus client, unit file parser, transition wait loop
└── config.rs    # XDG configuration loader and safe atomic writer
```

---

## 🤝 Contributing & License

Contributions, bug reports, and pull requests are warmly welcomed! Please ensure all PRs adhere to [Conventional Commits](https://www.conventionalcommits.org/).

This project is licensed under the [MIT License](LICENSE).
