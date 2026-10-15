# Changelog

All notable changes to **`uu-remote-linux`** (`uur`) are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-10-15

### Added
- **Zero-Trap Remote Cursor Hook (`hook/uur-hook.c`, `hook/selftest.c`)**:
  - Intercepted `USER32.dll!GetCursorInfo` and `USER32.dll!GetCursorPos` across both PE Import Address Tables (IAT) and dynamic `GetProcAddress` lookups in `GameViewerServer.exe` and `streamer.dll`.
  - Synthesized `CURSOR_SHOWING` (`0x00000001`), a valid `IDC_ARROW` cursor handle, and synchronized pointer coordinates so Windows UU Remote controllers never misclassify the Linux host as having a hidden/captured FPS game cursor (`ClipCursor` mouse lock).
  - Added `cursor_info=` diagnostic telemetry in `C:\uur-capture-status-<pid>.txt` and automated `GetCursorInfo` assertions in `hook/selftest.c`.
- **Multi-Version Launcher & NSIS Upgrade Self-Healing (`src/wine.rs`)**:
  - Added `repair_client_bin_link()` and `prepare_client_dir_for_install()` to automatically clean stale `<canonical>/bin` symlinks before running the NSIS installer and rebind `<canonical>/bin -> <highest_semver>` (`4.42.1.2835+`) when Wine NSIS upgrades rename older directories to `1.0.0.0`.
- **Forward-Compatible Official Client Verification (`src/client_pin.rs`)**:
  - Retained the SHA-256 fast-path pin for NetEase UU Remote `4.42.1.2835` (`d3a7776f6fb00b196e2fcbf8acba06fd52b19437b0da80fec0ae838ee362f06f`) while allowing validated PE (`MZ` header, $\ge 1\text{ MB}$) installers from the official NetEase feed when upstream publishes newer builds.
  - Added `UUR_STRICT_CLIENT_PIN=1` environment toggle for strict hash-only enforcement.
- **Universal Linux Release Packaging (`packaging/build-release.sh`, `.github/workflows/ci.yml`)**:
  - Unified package identity to `uu-remote-linux` `v0.2.0` (`Provides/Conflicts/Replaces: uur, uu-remote-mint`).
  - Added automated generation and verification of both `uu-remote-linux_0.2.0_amd64.deb` (Debian/Ubuntu/Linux Mint) and `uu-remote-linux-0.2.0-linux-x86_64.tar.zst` (universal portable archive for Arch, Fedora, openSUSE, etc.) with `SHA256SUMS`.

### Fixed
- **Wine Service Registration Detection (`src/wine.rs`)**:
  - Fixed `service_registered_in_system_reg()` and `query_service_registered()` to recognize `GameViewerService.exe` (in addition to `GameViewerServer.exe`) across both `str(2):` (`REG_EXPAND_SZ`) and quoted/unquoted `REG_SZ` registry formats.
  - Added bounded 60-second `wineserver -w` synchronization after NSIS setup and `sc.exe` registration so `system.reg` is flushed deterministically before verification.
- **Wine C++ Runtime Overrides (`src/wine.rs`)**:
  - Added `msvcp140` and `msvcp140_1` (`native,builtin`) AppDefaults overrides across all 5 UU executables (`gameviewer.exe`, `GameViewerServer.exe`, `GameViewerService.exe`, `GameViewerLauncher.exe`, `GameViewerHealthd.exe`).
- **X11 Session Detection & XTest Keysym Mapping (`src/input.rs`, `src/session.rs`, `src/doctor.rs`)**:
  - Prioritized `XDG_SESSION_TYPE=x11` over inherited `WAYLAND_DISPLAY` variables.
  - Verified `xtest::get_version` request cookies during X11 connection setup.
  - Fixed `VK_INSERT` (`0x2D` -> `0xff63`), `VK_DELETE` (`0x2E` -> `0xffff`), and generic `VK_SHIFT`/`VK_CONTROL`/`VK_MENU` mappings in `vk_to_keysym()`.
  - Prioritized executable-relative (`../lib/uur/hook`) and user (`~/.local/lib/uur/hook`) hook directories ahead of `/usr/lib/uur/hook` so local deployments always load the matching `uur-hook.dll`.

## [0.1.5] - Upstream Baseline

- Initial Linux compatibility layer architecture from [`panxuc/uur`](https://github.com/panxuc/uur) (`ef72baa56dde9daaa71a88d2bf4e2e14f6879fd4`).
