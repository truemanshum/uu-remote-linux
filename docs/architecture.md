# Architecture

`uur` is a compatibility layer, not a repackaged Windows application. The
official client remains in an isolated per-user Wine prefix. A small PE hook
adapts stable Windows APIs at runtime; the Rust host owns policy, desktop
integration, and process lifetime.

```text
UU Remote / Wine
  └─ PE ABI adapter (SendInput, GDI/DXGI capture entry points)
       └─ authenticated loopback protocol + private frame transport
            └─ Rust session supervisor
                 ├─ capture source: xdg ScreenCast → PipeWire
                 ├─ input sink (capability-selected)
                      ├─ xdg RemoteDesktop
                      ├─ Linux uinput
                      └─ X11 XTest
                 └─ native services
                      ├─ authenticated terminal proxy → Linux PTY
                      ├─ authenticated Quick Launch proxy → XDG applications
                      ├─ wallpaper metadata synchronizer
                      └─ desktop suspend/idle inhibitor
```

## Capability negotiation

No runtime branch checks a desktop or distribution name. The order is based on
available interfaces:

1. On Wayland, try a combined RemoteDesktop + ScreenCast portal session. This
   is the preferred unprivileged path and uses one consent session for video
   and input.
2. If the compositor exposes ScreenCast but not RemoteDesktop, keep portal
   capture and inject through `/dev/uinput`. This covers compositors whose
   portal currently implements capture only.
3. On X11, use XTest. Wine already sees the real X11 desktop, so an additional
   portal capture is unnecessary.
4. XTest remains a final partial fallback for XWayland windows.

Portal implementations are selected by `xdg-desktop-portal`, not by uur. This
keeps compositor-specific policy in the component designed to own it.

## Adapter boundary

The PE DLL is intentionally small C code because it must obey the Windows ABI.
It intercepts API imports and dynamic `GetProcAddress` resolution; it does not
search for product-version-specific instruction bytes or patch official
binaries. Everything above that ABI boundary is Rust.

Capture follows the same pattern as `wemeet-wayland-screenshare`: intercept a
stable capture boundary, obtain real compositor frames through PipeWire, and
substitute those frames before the proprietary application consumes them.
The implementation supports GDI today and keeps DXGI behind the same adapter
surface.

### Cursor state synthesis (`GetCursorInfo` / `GetCursorPos`)

Under Wine's X11 or headless server environment, `GameViewerServer.exe` and
`streamer.dll` query `USER32!GetCursorInfo` and `USER32!GetCursorPos` to detect
whether the remote desktop has a visible hardware cursor. When no foreground
Win32 window owns the cursor inside the Wine prefix, `GetCursorInfo` can report
`flags == 0` (cursor hidden) or `hCursor == NULL`, causing remote controllers to
enter relative-mouse capture mode (`ClipCursor`).

`uur-hook.dll` interposes `GetCursorInfo` and `GetCursorPos` across both PE
Import Address Tables (IAT) and `GetProcAddress` lookups, tracking pointer
coordinates from `SetCursorPos` and `SendInput(MOUSEEVENTF_MOVE)` and reporting
`CURSOR_SHOWING` with a valid `IDC_ARROW` handle so the remote controller
remains in normal desktop pointer mode.

### Multi-version Wine prefix reconciliation

When the official NSIS updater upgrades an existing installation inside Wine,
it renames the prior version directory to `1.0.0.0` and unpacks the new payload
into a versioned directory (`<major>.<minor>.<patch>.<build>`). Because the
top-level `GameViewer.exe` launcher and `GameViewerService.exe` resolve child
binaries through `<install_root>\bin\`, `uur` reconciles `<install_root>/bin` to
point to the highest semantic version directory before and after setup or
session startup.

## Lifetime and isolation

`uur run` is the only supervisor. It creates each native helper in a dedicated
process group and records the supervisor PID plus Linux process start time.
When the UU UI exits, or when `uur stop` sends SIGTERM to that exact supervisor,
RAII cleanup terminates only those helper groups and the one managed Wine
prefix. There are no permanent daemons, global `pkill` calls, or required
systemd units.

Frame data lives in `$XDG_RUNTIME_DIR/uur/frames.v1` with mode 0600. The path is
passed through the adapter configuration, so users and concurrent sessions do
not share a global `/dev/shm` object.

## Extension seams

- `InputBackend`: portal, uinput, XTest, and future libei transports.
- capture helper protocol: producer-independent triple-buffer transport.
- PE adapter: API-level UU integration isolated from Linux backends.
- session supervisor: process lifetime independent of init system.
- packaging stage: one filesystem layout consumed by AUR, deb, rpm, Arch, and
  portable tarball builders.
