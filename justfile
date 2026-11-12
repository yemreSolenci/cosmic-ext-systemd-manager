rootdir := ""
prefix := "/usr"
base-dir := rootdir + prefix
bindir := base-dir + "/bin"
datadir := base-dir + "/share"
app-id := "io.github.yemresolenci.CosmicExtAppletSystemdManager"
bin-name := "cosmic-ext-applet-systemd-manager"

# Default recipe: build release binary
default: build-release

build:
    cargo build

build-release:
    cargo build --release

install:
    install -Dm0755 "target/release/{{bin-name}}" "{{bindir}}/{{bin-name}}"
    install -Dm0644 "res/{{app-id}}.desktop" "{{datadir}}/applications/{{app-id}}.desktop"
    install -Dm0644 "res/{{app-id}}.metainfo.xml" "{{datadir}}/metainfo/{{app-id}}.metainfo.xml"
    install -Dm0644 "res/icons/hicolor/scalable/apps/{{app-id}}.svg" "{{datadir}}/icons/hicolor/scalable/apps/{{app-id}}.svg"

uninstall:
    rm -f "{{bindir}}/{{bin-name}}"
    rm -f "{{datadir}}/applications/{{app-id}}.desktop"
    rm -f "{{datadir}}/metainfo/{{app-id}}.metainfo.xml"
    rm -f "{{datadir}}/icons/hicolor/scalable/apps/{{app-id}}.svg"
