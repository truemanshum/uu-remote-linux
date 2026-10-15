# Package Provenance & Binary Boundary

## 1. Source & Package Identity

- **Project / Repository**: [`truemanshum/uu-remote-linux`](https://github.com/truemanshum/uu-remote-linux)
- **Version**: `0.2.0`
- **Artifacts**:
  - `uu-remote-linux_0.2.0_amd64.deb` (Debian / Ubuntu / Linux Mint package; `Provides/Conflicts/Replaces: uur, uu-remote-mint`)
  - `uu-remote-linux-0.2.0-linux-x86_64.tar.zst` (Universal Linux `x86_64` portable archive)
- **License**: MIT (`LICENSE` and `NOTICE` included in all packages)

All binaries in the release artifacts are compiled from source in CI from the locked Rust dependency graph (`Cargo.lock`), C11 capture helpers (`capture/`), and MinGW-w64 PE64 hook modules (`hook/`).

## 2. Dynamic Linking & ABI Footprint

Static ELF64 and PE64 header inspection confirms that the compiled artifacts have minimal host library coupling and no desktop-specific shared library dependencies:

| Binary Path | Format | Hardening | Dynamic Dependencies (`DT_NEEDED` / Imports) |
| --- | --- | --- | --- |
| `/usr/bin/uur` | `ELF64 amd64 DYN (PIE)` | Full RELRO, NX | `libc.so.6`, `libm.so.6`, `libgcc_s.so.1` (`GLIBC_2.39`) |
| `/usr/lib/uur/uur-pw-capture` | `ELF64 amd64 DYN (PIE)` | Full RELRO, NX, Canary | `libpipewire-0.3.so.0`, `libc.so.6` |
| `/usr/lib/uur/uur-wl-capture` | `ELF64 amd64 DYN (PIE)` | Full RELRO, NX, Canary | `libwayland-client.so.0`, `libc.so.6` |
| `/usr/lib/uur/hook/uur-hook.dll` | `PE64 x86_64 DLL` | ASLR, DEP | `KERNEL32.dll`, `USER32.dll`, `GDI32.dll`, `WS2_32.dll`, `msvcrt.dll` |
| `/usr/lib/uur/hook/wevtapi.dll` | `PE64 x86_64 DLL` | ASLR, DEP | `uur-hook.dll`, `KERNEL32.dll`, `msvcrt.dll` |
| `/usr/lib/uur/hook/wtsapi32.dll` | `PE64 x86_64 DLL` | ASLR, DEP | `ADVAPI32.dll`, `KERNEL32.dll`, `msvcrt.dll` |

## 3. Proprietary Client Boundary

`uu-remote-linux` does not redistribute proprietary NetEase software. During `uur setup --accept-eula`, the official Windows installer is downloaded directly from NetEase into the invoking user's XDG cache.

Pinned fast-path client manifest (`src/client_pin.rs`):

| Field | Value |
| --- | --- |
| Product version | `4.42.1.2835` |
| Size | `89664288` bytes |
| SHA-256 | `d3a7776f6fb00b196e2fcbf8acba06fd52b19437b0da80fec0ae838ee362f06f` |

When NetEase updates the official CDN feed to a newer build, `src/client_pin.rs` validates the downloaded payload size ($\ge 1\text{ MB}$) and Windows PE (`MZ`) header before permitting installation (unless `UUR_STRICT_CLIENT_PIN=1` is set).

## 4. Reproducibility Controls

- Rust compiler version is pinned via `rust-toolchain.toml` (`1.99.0`) and built with `--locked`.
- `SOURCE_DATE_EPOCH` defaults to the Git commit timestamp, and all staged file mtimes and tar headers are normalized prior to packaging.
- Every release produces `SHA256SUMS` verified in CI.

## 5. Upstream References

- Upstream architecture baseline: [`panxuc/uur`](https://github.com/panxuc/uur) (`ef72baa56dde9daaa71a88d2bf4e2e14f6879fd4`)
- Historical reference projects: [`GuoWQ222/uu-remote-for-linux`](https://github.com/GuoWQ222/uu-remote-for-linux) and [`lachlanchen/uu-remote-ubuntu-bridge`](https://github.com/lachlanchen/uu-remote-ubuntu-bridge)
