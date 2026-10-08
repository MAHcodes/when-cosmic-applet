default: build-release

destdir := env_var_or_default("DESTDIR", "")
prefix := env_var_or_default("PREFIX", "/usr/local")
# escalate only when a regular user installs into a system prefix
installer := if env_var_or_default("USER", "") == "root" { "" } else if destdir == "" { "sudo " } else { "" }

build:
    cargo build

build-release:
    cargo build --release

install: build-release
    {{installer}}install -Dm755 target/release/when-cosmic-applet "{{destdir}}{{prefix}}/bin/when-cosmic-applet"
    {{installer}}install -Dm644 com.ali.WhenPrayerApplet.desktop "{{destdir}}{{prefix}}/share/applications/com.ali.WhenPrayerApplet.desktop"

uninstall:
    {{installer}}rm -f "{{destdir}}{{prefix}}/bin/when-cosmic-applet"
    {{installer}}rm -f "{{destdir}}{{prefix}}/share/applications/com.ali.WhenPrayerApplet.desktop"

run:
    cargo run

check:
    cargo clippy
