default: build-release

build:
    cargo build

build-release:
    cargo build --release

install: build-release
    install -Dm755 target/release/when-cosmic-applet "${DESTDIR}${PREFIX}/bin/when-cosmic-applet"
    install -Dm644 com.ali.WhenPrayerApplet.desktop "${DESTDIR}${PREFIX}/share/applications/com.ali.WhenPrayerApplet.desktop"

uninstall:
    rm -f "${DESTDIR}${PREFIX}/bin/when-cosmic-applet"
    rm -f "${DESTDIR}${PREFIX}/share/applications/com.ali.WhenPrayerApplet.desktop"

run:
    cargo run

check:
    cargo clippy
