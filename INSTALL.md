# Installation Guide

This guide covers installing and configuring **`uu-remote-linux`** (`v0.2.0`) across Debian/Ubuntu/Linux Mint, Arch Linux, Fedora/RHEL/openSUSE, NixOS, and other Linux distributions.

---

## 1. Download Release Artifacts

Download the latest packages from [**GitHub Releases**](https://github.com/truemanshum/uu-remote-linux/releases/latest):

- `uu-remote-linux_0.2.0_amd64.deb` — Debian 13+, Ubuntu 24.04+ LTS, Linux Mint 22.x, Pop!_OS, Zorin OS, Deepin
- `uu-remote-linux-0.2.0-linux-x86_64.tar.zst` — Universal portable archive for Arch Linux, Fedora, openSUSE, Gentoo, Void Linux, etc.
- `SHA256SUMS` — SHA-256 checksum manifest

---

## 2. Debian, Ubuntu & Linux Mint (`.deb`)

NetEase UU Remote's installer wrapper is a 32-bit PE32 executable (`uu-remote-setup.exe`), while the installed application and services (`GameViewer.exe`, `GameViewerService.exe`, `GameViewerServer.exe`) are 64-bit PE32+ binaries. Enable `i386` multiarch support once if it is not already enabled:

```bash
sudo dpkg --add-architecture i386
sudo apt update
```

Verify the SHA-256 checksum and install the package:

```bash
sha256sum --check uu-remote-linux_0.2.0_amd64.deb.sha256
sudo apt install ./uu-remote-linux_0.2.0_amd64.deb
```

### Wine Package Compatibility

The `.deb` dependency metadata supports both distribution Wine packages (`wine`, `wine64`, `wine32:i386`) and official WineHQ builds (`winehq-staging`, `winehq-stable`, `winehq-devel`). Installing `uu-remote-linux` will never downgrade or remove an existing WineHQ installation.

If you are upgrading from an earlier `uur` or `uu-remote-mint` package, `apt` automatically replaces the previous package while preserving your `~/.local/share/uur` Wine prefix and account session.

---

## 3. Arch Linux, Fedora, openSUSE & Other Distros (`.tar.zst`)

Extract the portable archive into `/` (installs `/usr/bin/uur`, `/usr/lib/uur/*`, desktop entry, uinput rules, and icons):

```bash
sha256sum --check uu-remote-linux-0.2.0-linux-x86_64.tar.zst.sha256
sudo tar --zstd -C / -xf ./uu-remote-linux-0.2.0-linux-x86_64.tar.zst
```

Ensure `wine` (with 32-bit and 64-bit support), `pipewire` (for Wayland sessions), `curl`, and CJK fonts (`noto-fonts-cjk` / `google-noto-sans-cjk-ttc-fonts`) are installed via your distribution's package manager.

---

## 4. First-Time Setup & Launch

Run the following three commands in a desktop terminal:

```bash
# 1. Verify environment health (Wine, X11/Wayland, audio, and input backends)
uur doctor

# 2. Accept the NetEase EULA, download the official installer, and initialize the Wine prefix
uur setup --accept-eula

# 3. Start the UU Remote session
uur run
```

Once initialized, you can launch **UU Remote** directly from your desktop application menu or enable automatic startup on login:

```bash
uur autostart enable
```

---

## 5. Build from Source

To build both `uu-remote-linux_0.2.0_amd64.deb` and `uu-remote-linux-0.2.0-linux-x86_64.tar.zst` from source:

```bash
bash packaging/build-release.sh
bash tests/verify-package.sh dist/uu-remote-linux_0.2.0_amd64.deb
```

The build script compiles the Rust supervisor (`cargo build --release --locked`), the MinGW-w64 Win64 runtime hooks (`hook/build.sh`), and the native Linux capture helpers (`capture/build.sh`), normalizes timestamps via `SOURCE_DATE_EPOCH`, and verifies `dist/SHA256SUMS`.
