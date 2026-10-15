<p align="center">
  <img src="packaging/uur-icon.png" width="112" alt="UU Remote for Linux">
</p>

<h1 align="center">uu-remote-linux (<code>uur</code>)</h1>

<p align="center">
  <strong>Universal Native Linux Compatibility Engine for NetEase UU Remote — X11 &amp; Wayland</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Version-0.2.0-2ea44f?style=flat-square" alt="Version 0.2.0">
  <img src="https://img.shields.io/badge/Linux-x86__64-FCC624?style=flat-square&logo=linux&logoColor=black" alt="Linux x86_64">
  <img src="https://img.shields.io/badge/Display-X11%20%7C%20Wayland-4A90E2?style=flat-square" alt="X11 and Wayland">
  <img src="https://img.shields.io/badge/Distros-Ubuntu%20%7C%20Debian%20%7C%20Mint%20%7C%20Arch%20%7C%20Fedora-E95420?style=flat-square" alt="Supported Linux Distributions">
  <img src="https://img.shields.io/badge/Language-Rust%20%2B%20C11-DEA584?style=flat-square&logo=rust&logoColor=black" alt="Rust and C11">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="MIT License"></a>
</p>

<p align="center">
  <strong>English</strong> · <a href="README.zh-CN.md">简体中文</a> · <a href="CHANGELOG.md">Changelog</a> · <a href="INSTALL.md">Installation Guide</a>
</p>

> [!IMPORTANT]
> **Clean Open-Source Boundary**: `uu-remote-linux` (`uur`) is a pure open-source compatibility layer and supervisor. It **never** bundles or redistributes proprietary NetEase binaries. The official Windows client is fetched directly from NetEase servers only after the user explicitly accepts the official EULA via `uur setup --accept-eula`.

---

## 🌟 Overview

**`uu-remote-linux`** (`uur` `v0.2.0`) enables Linux workstations and servers across **X11** and **Wayland** desktop environments to run the official NetEase UU Remote (`4.42.1.2835+`) client both as a **controller** (controlling remote Windows/macOS/Linux machines) and as a **controlled host** (allowing Windows, macOS, iOS, or Android devices to remotely control this Linux desktop with low-latency video, audio, keyboard/mouse input, file transfer, and native PTY terminal access).

Instead of relying on fragile binary byte-patching or distribution allowlists, `uu-remote-linux` combines a **zero-desktop-dependency Rust supervisor (`uur`)**, **native PipeWire / Wayland capture helpers**, and a **lightweight Win64 PE API hook (`uur-hook.dll`)** inside an isolated, managed Wine prefix.

---

## 🚀 What's New in `v0.2.0` (Key Engineering Highlights)

| Area | Engineering Upgrade in `v0.2.0` | Problem Solved |
| --- | --- | --- |
| **Zero-Trap Remote Cursor (`uur-hook.dll`)** | Hooks `USER32!GetCursorInfo` and `USER32!GetCursorPos` across IAT and `GetProcAddress`, synthesizing `CURSOR_SHOWING` + valid `IDC_ARROW` cursor state and synchronized screen coordinates. | Eliminates the Windows controller FPS relative-mouse trap (`ClipCursor`), allowing the remote mouse to freely leave the UU window, click minimize/maximize/close buttons, and switch desktop windows. |
| **Multi-Version Launcher Auto-Healing (`src/wine.rs`)** | Cleans stale `bin` symlinks before NSIS upgrades and automatically reconciles `<install_root>/bin -> <highest_semver>` (`4.42.1.2835+`). | Fixes silent `GameViewer.exe` exit (`code 0`) and `GameViewerService.exe` launch failures when UU upgrades from `4.41.x` to `4.42.x` under Wine (where NSIS renames the old build to `1.0.0.0`). |
| **Wine Service & C++ Runtime Provisioning (`src/wine.rs`)** | Parses both `str(2):` (`REG_EXPAND_SZ`) and quoted/unquoted `REG_SZ` `ImagePath` entries for `GameViewerService.exe`, enforces bounded 60s `wineserver -w` barriers, and provisions `msvcp140` + `msvcp140_1` (`native,builtin`) across all 5 UU binaries. | Fixes false `GameViewerService is not registered` failures on Wine 9–11 and prevents `GameViewerServer.exe` / `GameViewerHealthd.exe` startup crashes. |
| **Authoritative X11 & XTest Input (`src/input.rs`, `src/session.rs`)** | Prioritizes `XDG_SESSION_TYPE=x11` over inherited socket variables, verifies `xtest::get_version` cookies, maps `Insert`/`Delete` and generic `Shift`/`Ctrl`/`Alt` virtual keys, and prioritizes executable-relative / user `~/.local/lib/uur/hook` paths. | Ensures 100% reliable keyboard shortcuts, text editing keys, and pointer injection on Cinnamon, XFCE, MATE, GNOME Xorg, KDE X11, and tiling WMs. |
| **Forward-Compatible Client Verification (`src/client_pin.rs`)** | Retains SHA-256 fast-path verification for `4.42.1.2835` while allowing validated PE (`MZ` header, $\ge 1\text{ MB}$) installers when NetEase updates its CDN feed (`UUR_STRICT_CLIENT_PIN=1` enforces strict hash pinning). | Prevents `uur setup --accept-eula` from breaking the moment NetEase publishes a new minor build. |
| **Universal Linux Packaging (`packaging/`)** | Produces both a universal Debian/Ubuntu/Mint package (`uu-remote-linux_0.2.0_amd64.deb`, replacing `uur` and `uu-remote-mint`) and a distro-agnostic portable archive (`uu-remote-linux-0.2.0-linux-x86_64.tar.zst`). | Ready for immediate deployment on Debian, Ubuntu, Linux Mint, Pop!_OS, Arch Linux, Fedora, openSUSE, and NixOS. |

---

## 🖥️ Supported Linux Distributions & Desktop Environments

`uu-remote-linux` detects capabilities at runtime rather than hardcoding distro names:

| Session Type | Desktop Environments / Window Managers | Video Capture Path | Input Injection Path |
| --- | --- | --- | --- |
| **X11 (`Xorg`)** | Linux Mint **Cinnamon**, **MATE**, **XFCE**, Ubuntu/GNOME Xorg, KDE Plasma X11, i3, bspwm, Awesome, Openbox | Wine X11 desktop GDI/DXGI (`BitBlt` / `StretchBlt`) + cursor overlay | X11 **XTest** extension (unprivileged, no `/dev/uinput` required) |
| **Wayland (Portal)** | **KDE Plasma**, **GNOME**, Cosmic, Hyprland, Sway | XDG `ScreenCast` Portal → **PipeWire** (`uur-pw-capture`) → shared-memory triple buffer | XDG `RemoteDesktop` Portal or `/dev/uinput` |
| **Wayland (Direct)** | **Niri**, wlroots compositors (`wlr-screencopy`) | `zwlr_screencopy_manager_v1` (`uur-wl-capture`) | `/dev/uinput` |

### Binary ABI Footprint

Static ELF64 and PE64 symbol inspection confirms minimal host library coupling:

- `/usr/bin/uur` (`ELF64 PIE`, Full RELRO, NX): links **only** `libc.so.6`, `libm.so.6`, and `libgcc_s.so.1` (`GLIBC_2.39+`). X11 (`x11rb`) and D-Bus (`zbus`) protocols are implemented in pure Rust without linking `libX11` or `libgtk`.
- `/usr/lib/uur/uur-pw-capture` (`ELF64 PIE`): links `libpipewire-0.3.so.0` and `libc.so.6`.
- `/usr/lib/uur/uur-wl-capture` (`ELF64 PIE`): links `libwayland-client.so.0` and `libc.so.6`.
- `/usr/lib/uur/hook/*.dll` & `*.exe` (`PE64 x86_64`): self-contained MinGW-w64 runtime adapters loaded inside Wine (`KERNEL32.dll`, `USER32.dll`, `GDI32.dll`, `WS2_32.dll`, `ADVAPI32.dll`, `msvcrt.dll`).

---

## 📦 Installation

Download the latest release artifacts from [**GitHub Releases**](https://github.com/truemanshum/uu-remote-linux/releases/latest):

- **`uu-remote-linux_0.2.0_amd64.deb`** — Debian 13+, Ubuntu 24.04+, Linux Mint 22.x, Pop!_OS, Zorin OS
- **`uu-remote-linux-0.2.0-linux-x86_64.tar.zst`** — Universal portable archive for Arch Linux, Fedora, openSUSE, Gentoo, Void Linux, etc.
- **`SHA256SUMS`** — SHA-256 checksum manifest

### 1. Debian, Ubuntu, and Linux Mint (`.deb`)

Because NetEase's official installer wrapper is a 32-bit PE32 executable while the installed UU Remote payload (`GameViewer.exe`, `GameViewerServer.exe`, `GameViewerService.exe`) is 64-bit PE32+, ensure 32-bit Wine support is enabled once:

```bash
# Enable 32-bit architecture for the PE32 installer wrapper (if not already enabled)
sudo dpkg --add-architecture i386
sudo apt update

# Verify checksum and install the package
sha256sum --check uu-remote-linux_0.2.0_amd64.deb.sha256
sudo apt install ./uu-remote-linux_0.2.0_amd64.deb
```

> **Note**: `uu-remote-linux` declares `Provides/Conflicts/Replaces: uur, uu-remote-mint` and supports standard distribution `wine` packages as well as `winehq-staging`, `winehq-stable`, and `winehq-devel` without package conflicts.

### 2. Arch Linux, Fedora, openSUSE & Other Distros (Portable `.tar.zst`)

Extract the portable archive directly into `/` (installs `/usr/bin/uur`, `/usr/lib/uur/*`, desktop launcher, and icons):

```bash
sha256sum --check uu-remote-linux-0.2.0-linux-x86_64.tar.zst.sha256
sudo tar --zstd -C / -xf ./uu-remote-linux-0.2.0-linux-x86_64.tar.zst
```

Make sure `wine` (with 32-bit + 64-bit support), `pipewire` (for Wayland), `curl`, and CJK fonts (`noto-fonts-cjk` / `google-noto-sans-cjk-ttc-fonts`) are installed via your distribution's package manager.

### 3. Build & Install from Source

```bash
# Build Universal .deb and .tar.zst packages into ./dist/
bash packaging/build-release.sh

# Or install directly into ~/.local for the current user
cargo build --release --locked
./hook/build.sh
./capture/build.sh
./scripts/install-user.sh
```

---

## ⚡ Quick Start (First Run)

Run these three commands in your desktop terminal (or launch **UU Remote** from your desktop application menu after setup):

```bash
# 1. Inspect Wine, display server (X11/Wayland), audio, and input readiness
uur doctor

# 2. Accept NetEase's official EULA, download the official client, and initialize the Wine prefix
uur setup --accept-eula

# 3. Start the capture/input bridge, managed Wine services, and UU Remote UI
uur run
```

---

## 🧭 Daily CLI Reference

```bash
uur run                          # Start or focus the managed UU Remote session
uur doctor                       # Run read-only environment & health diagnostics
uur stop                         # Cleanly stop UU Remote helpers and the managed Wine prefix
uur autostart enable             # Automatically launch UU Remote on desktop login
uur autostart disable            # Remove the XDG autostart entry
uur upstream check               # Query the official NetEase release feed for newer versions
uur wol status                   # Inspect Wake-on-LAN status on the physical wired NIC
uur display status               # Inspect active capture backend and display source
uur uninstall --purge --dry-run  # Preview complete removal of user Wine prefix & cache
uur uninstall --purge            # Remove user Wine prefix, logs, and cached installers
```

### Environment Variables (Advanced / Diagnostics)

| Variable | Values | Description |
| --- | --- | --- |
| `UUR_INPUT_BACKEND` | `auto` (default), `xtest`, `portal`, `uinput` | Override input injection backend for diagnostics |
| `UUR_STRICT_CLIENT_PIN` | `0` (default), `1` | When set to `1`, require exact SHA-256 match against the pinned installer manifest |
| `WINEDEBUG` | e.g. `+loaddll` | Enable verbose Wine loader logs (`fixme` noise is suppressed by default) |

---

## 🔬 Architecture

```text
┌───────────────────────────────────────────────────────────────────────────┐
│                Official NetEase UU Remote Client (Wine64)                 │
│  GameViewer.exe (UI) · GameViewerService.exe · GameViewerServer.exe       │
├───────────────────────────────────┬───────────────────────────────────────┤
│   PE64 Runtime Hook (uur-hook.dll)│   PE64 Native Service Proxies         │
│   • SendInput / SetCursorPos      │   • uur-terminal-proxy.exe (PTY)      │
│   • GetCursorInfo / GetCursorPos  │   • uur-launch-proxy.exe (XDG Launch) │
│   • BitBlt / StretchBlt / DXGI    │   • uur-mux-proxy.exe (File/Clipboard)│
└─────────────────┬─────────────────┴───────────────────┬───────────────────┘
                  │ Authenticated Local TCP + SHM       │
┌─────────────────▼─────────────────────────────────────▼───────────────────┐
│                     Native Linux Supervisor (uur - Rust)                  │
├───────────────────────────────────┬───────────────────────────────────────┤
│   Display & Capture Backends      │   Input & Native OS Adapters          │
│   • X11: Native Wine X11 + Cursor │   • X11: Pure-Rust XTest Injection    │
│   • Wayland: ScreenCast + PipeWire│   • Wayland: RemoteDesktop / uinput   │
│   • wlroots: wlr-screencopy       │   • Native PTY, Gio Launch, WOL, D-Bus│
└───────────────────────────────────┴───────────────────────────────────────┘
```

---

## 📚 Documentation

- [**Installation & Distribution Guide**](INSTALL.md)
- [**Changelog & Release History**](CHANGELOG.md)
- [**System Architecture**](docs/architecture.md)
- [**Platform Support Matrix**](docs/platform-support.md)
- [**Package Provenance & Reproducibility**](docs/provenance.md)
- [**Capture Frame Protocol**](docs/capture-protocol.md)
- [**Native Linux Service Adapters**](docs/native-services.md)

---

## 🙏 Credits & Upstream References

This project is engineered and maintained for universal Linux desktop compatibility and draws upon the work of the open-source community:

- Upstream architecture foundation: [`panxuc/uur`](https://github.com/panxuc/uur) (`v0.1.5`, MIT License)
- Early Linux Wine & bridge research:
  - [`GuoWQ222/uu-remote-for-linux`](https://github.com/GuoWQ222/uu-remote-for-linux)
  - [`lachlanchen/uu-remote-ubuntu-bridge`](https://github.com/lachlanchen/uu-remote-ubuntu-bridge)

**Disclaimer**: NetEase UU Remote (网易UU远程 / GameViewer) is proprietary software owned by NetEase, Inc. `uu-remote-linux` is an independent open-source compatibility layer and is not affiliated with or endorsed by NetEase.
