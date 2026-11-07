# COSMIC Systemd Manager

A native, highly polished `libcosmic` panel applet for the Pop!_OS COSMIC desktop environment that allows you to monitor and control your favorite `systemd` services directly from your top bar.

Inspired by popular GNOME extensions, this applet provides a deeply integrated, lightning-fast UX for managing the daemons you care about without ever touching a terminal.

---

## ✨ Features

- **Real-Time D-Bus Integration**: Directly communicates with systemd over `zbus`, fetching live unit states and broadcasting start/stop/restart commands instantly.
- **Persistent Pinned Services**: Don't scroll through 200 system units. Pin only the services you care about (like `docker.service` or `vban.service`). Configuration survives reboots, saved to `~/.config/systemdmanager/config.json`.
- **Custom Aliases/Labels**: Rename ugly system unit IDs into clean, human-readable labels for your panel.
- **Service Reordering**: Move your pinned services up and down inside the Manage view to fit your workflow.
- **Live-Search Autocomplete**: Adding a new service? Just type a few letters and the applet will live-filter your system's unit registry, ensuring you only add valid targets.
- **Destructive Action Protections**: Styled error messages, native duplicate checking, and confirmation flows prevent accidentally adding broken or duplicate services.
- **Keyboard-Friendly**: Auto-focus on label input fields so you can navigate the entire flow without touching your mouse.

---

## 🚀 Installation

### Prerequisites

Ensure you have the Rust toolchain installed. Install it via [rustup](https://rustup.rs) if needed:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

You will also need the development libraries for your distribution. On Pop!_OS / Ubuntu:
```bash
sudo apt install libwayland-dev libxkbcommon-dev libdbus-1-dev pkg-config
```

### 1. Clone and Build

```bash
git clone https://github.com/yemreSolenci/cosmic-systemd-manager.git
cd cosmic-systemd-manager
cargo build --release
```

The compiled binary will be at `target/release/cosmic-systemd-applet`.

### 2. Install the Binary

Move it somewhere on your `$PATH`. The `~/.local/bin` directory is the standard user-local location:

```bash
mkdir -p ~/.local/bin
cp target/release/cosmic-systemd-applet ~/.local/bin/
```

Verify it is on your path (add to `~/.bashrc` or `~/.profile` if needed):
```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### 3. Register the Applet with COSMIC Panel

COSMIC panel discovers applets through `.desktop` files with the `X-CosmicApplet=true` key. The file **must** be placed in a system-wide or user-level data directory.

Create the desktop entry file:
```bash
mkdir -p ~/.local/share/applications/
```

Create `~/.local/share/applications/io.github.yemresolenci.CosmicSystemdManager.desktop` with the following content:

```ini
[Desktop Entry]
Name=Systemd Manager
Comment=Manage pinned systemd services from the panel
Type=Application
Exec=cosmic-systemd-applet
Terminal=false
Categories=COSMIC;System;
Keywords=COSMIC;Applet;Systemd;Services;
Icon=io.github.yemresolenci.CosmicSystemdManager
NoDisplay=true
X-CosmicApplet=true
```

> **Important**: The `Exec=` value must match the binary name as it appears on your `$PATH` (just `cosmic-systemd-applet`, not an absolute path). This is how COSMIC panel launches applets.

> **Note**: The `NoDisplay=true` key hides the applet from the regular app launcher — it should only appear in the COSMIC Panel configuration interface, just like system applets.

### 4. Add the Applet to Your Panel

After creating the `.desktop` file, log out and back in (or restart `cosmic-panel`):
```bash
killall cosmic-panel
```

Then go to **COSMIC Settings → Desktop → Panel → Add Applet** and you should see **"Systemd Manager"** in the list.

---

## 🔐 Polkit Permissions

By default, standard users are prompted for a password when starting or stopping **system-wide** systemd units (like `docker.service`). To make the panel applet seamless — no password popups on every click — configure a Polkit rule.

**Create the rule file:**
```bash
sudo nano /etc/polkit-1/rules.d/49-nopasswd-systemd.rules
```

**Paste the following** *(replace `YOUR_USERNAME` with your actual Linux username)*:
```javascript
polkit.addRule(function(action, subject) {
    if (action.id == "org.freedesktop.systemd1.manage-units" &&
        subject.user == "YOUR_USERNAME") {
        return polkit.Result.YES;
    }
});
```

Restart the polkit service to apply:
```bash
sudo systemctl restart polkit
```

> ⚠️ **Security Note**: The rule above grants your user password-less control over ALL systemd units. For a more restrictive, least-privilege setup, scope the rule to an explicit allowlist of units using `action.lookup("unit")`:
>
> ```javascript
> polkit.addRule(function(action, subject) {
>     var allowedUnits = ["docker.service", "vban.service"];
>     if (action.id == "org.freedesktop.systemd1.manage-units" &&
>         subject.user == "YOUR_USERNAME" &&
>         allowedUnits.indexOf(action.lookup("unit")) !== -1) {
>         return polkit.Result.YES;
>     }
> });
> ```

---

## ⚙️ Configuration

Pinned services and their labels are automatically persisted to:
```
~/.config/systemdmanager/config.json
```

The format is a simple JSON array and can be hand-edited if needed:
```json
[
  { "id": "docker.service", "label": "Docker" },
  { "id": "vban.service",   "label": "VBAN Audio" }
]
```

---

## 🛠 Architecture & Tech Stack

| Crate | Purpose |
|---|---|
| `libcosmic` | Declarative GUI, Wayland layer-shell protocol, panel applet lifecycle |
| `zbus` | Async D-Bus communication with `org.freedesktop.systemd1` |
| `tokio` | Async runtime |
| `serde` + `serde_json` | Configuration serialization |
| `directories` | XDG-compliant config path resolution |

### Module Structure

```
src/
├── main.rs      # Entry point — calls cosmic::applet::run
├── app.rs       # Application state, message types, update logic
├── ui.rs        # Pure declarative view functions per screen
├── config.rs    # Load/save pinned services to disk
└── systemd.rs   # D-Bus client for org.freedesktop.systemd1
```

---

## 📄 License

This project is open-source and available under the [MIT License](LICENSE). Feel free to fork, extend, and contribute!
