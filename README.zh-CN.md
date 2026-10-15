<p align="center">
  <img src="packaging/uur-icon.png" width="112" alt="UU 远程 Linux 版图标">
</p>

<h1 align="center">uu-remote-linux (<code>uur</code>)</h1>

<p align="center">
  <strong>适用于全系 Linux 桌面（X11 &amp; Wayland）的网易 UU 远程原生兼容层与运行时引擎</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Version-0.2.0-2ea44f?style=flat-square" alt="版本 0.2.0">
  <img src="https://img.shields.io/badge/Linux-x86__64-FCC624?style=flat-square&logo=linux&logoColor=black" alt="Linux x86_64">
  <img src="https://img.shields.io/badge/Display-X11%20%7C%20Wayland-4A90E2?style=flat-square" alt="X11 与 Wayland">
  <img src="https://img.shields.io/badge/Distros-Ubuntu%20%7C%20Debian%20%7C%20Mint%20%7C%20Arch%20%7C%20Fedora-E95420?style=flat-square" alt="全系主流发行版">
  <img src="https://img.shields.io/badge/Language-Rust%20%2B%20C11-DEA584?style=flat-square&logo=rust&logoColor=black" alt="Rust 与 C11">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="MIT 许可证"></a>
</p>

<p align="center">
  <a href="README.md">English</a> · <strong>简体中文</strong> · <a href="CHANGELOG.md">更新日志</a> · <a href="INSTALL.md">安装指南</a>
</p>

> [!IMPORTANT]
> **开源合规边界**：`uu-remote-linux`（命令行工具 `uur`）仅提供开源的 Linux 兼容层、画面/输入桥接器与 Wine 运行时钩子，**不包含、不分发**任何网易专有客户端程序。只有在用户显式执行 `uur setup --accept-eula` 接受网易官方用户协议后，程序才会从网易官方服务器下载安装程序。

---

## 🌟 项目简介

**`uu-remote-linux`** (`uur` `v0.2.0`) 让运行在 **X11** 或 **Wayland** 会话下的各类 Linux 发行版（**Ubuntu、Debian、Linux Mint、Pop!_OS、Deepin、Zorin、Arch Linux、Fedora、openSUSE、NixOS** 等）能够完整运行网易 UU 远程（`4.42.1.2835+`）：

- **作为主控端**：在 Linux 桌面上直接远控其他 Windows、macOS 或 Linux 设备；
- **作为被控端**：让其他 Windows、macOS、iOS 或 Android 设备远控当前这台 Linux 电脑，支持低延迟桌面画面采集、键盘与鼠标自由操控、远程原生终端（PTY）、文件传输、剪贴板同步与快速启动本地应用。

---

## 🚀 `v0.2.0` 核心工程升级亮点

针对最新官方客户端（`4.42.1.2835+`）与主流 Linux 桌面环境（X11 与 Wayland、Wine 9–11 / WineHQ Staging），`v0.2.0` 在系统兼容层与运行时边界上完成了六项关键工程升级：

| 核心模块 | `v0.2.0` 架构升级细节 | 解决的实际问题 |
| --- | --- | --- |
| **防鼠标锁死引擎 (`uur-hook.dll`)** | 拦截 `GameViewerServer.exe` 与 `streamer.dll` 对 `USER32!GetCursorInfo` 和 `GetCursorPos` 的 IAT 导入表与 `GetProcAddress` 动态调用，始终合成 `CURSOR_SHOWING`、有效 `IDC_ARROW` 句柄与实时屏幕坐标。 | **彻底解决 Windows 主控端连接 Linux 后鼠标被锁死在远控窗口内的问题**（不再误判为隐藏光标的 FPS 游戏模式而触发 `ClipCursor`），鼠标可自由移出窗口、点击最小化/全屏/关闭按钮。 |
| **多版本升级自愈 (`src/wine.rs`)** | 安装前自动清理陈旧符号链接，并在启动时按语义化版本号自动将 `<安装目录>/bin` 指向最高版本目录（如 `4.42.1.2835`）。 | **解决客户端从 `4.41` 升级到 `4.42` 后无法启动的问题**（NSIS 升级器在 Wine 下会将旧目录重命名为 `1.0.0.0` 并导致 `bin` 符号链接悬空、主程序秒退）。 |
| **服务注册与运行库完备化 (`src/wine.rs`)** | 同时支持 `str(2):` (`REG_EXPAND_SZ`) 与带引号/不带引号的 `REG_SZ` 注册表 `ImagePath` 解析，增加 60 秒有界 `wineserver -w` 同步屏障，并为全部 5 个核心 EXE 注入 `msvcp140` / `msvcp140_1` (`native,builtin`)。 | **解决 `GameViewerService` 误报未注册及 `GameViewerServer.exe` 缺少 C++ 运行库崩溃退出的问题**。 |
| **X11 / XTest 精准键鼠注入 (`src/input.rs`, `src/session.rs`)** | 优先采信 `XDG_SESSION_TYPE=x11`，严格校验 `xtest::get_version` 握手，补齐 `Insert`、`Delete` 与通用 `Shift`/`Ctrl`/`Alt` 修饰键映射，并优先加载可执行文件同级或 `~/.local/lib/uur/hook` 钩子目录。 | **确保在 Cinnamon、XFCE、MATE、GNOME Xorg、KDE X11 等桌面下键鼠操作与快捷键 100% 精准响应**。 |
| **官方客户端平滑升级校验 (`src/client_pin.rs`)** | 保留 `4.42.1.2835` SHA-256 快速通道，同时当网易官方发布新版本时自动验证 PE (`MZ`) 可执行头与文件体积后平滑放行（支持通过 `UUR_STRICT_CLIENT_PIN=1` 开启严格哈希锁死）。 | **防止网易官方更新安装包后导致新用户执行 `uur setup --accept-eula` 报错失效**。 |
| **全系 Linux 通用打包 (`packaging/`)** | 一次构建同时产出 Debian/Ubuntu/Mint 专用包 (`uu-remote-linux_0.2.0_amd64.deb`，自动无缝替换旧版 `uur` 与 `uu-remote-mint`) 和跨发行版通用包 (`uu-remote-linux-0.2.0-linux-x86_64.tar.zst`)。 | **一套代码与产物通吃 Debian 系、Arch 系、RedHat/Fedora 系与其它主流 Linux 发行版**。 |

---

## 🖥️ 跨发行版与桌面环境支持矩阵

`uu-remote-linux` 在运行时自动探测图形会话与系统能力，无需针对特定发行版硬编码白名单：

| 图形会话类型 | 支持的桌面环境 / 窗口管理器 | 画面采集通道 | 键鼠输入注入通道 |
| --- | --- | --- | --- |
| **X11 (`Xorg`)** | Linux Mint **Cinnamon** / **MATE** / **XFCE**、Ubuntu GNOME Xorg、KDE Plasma X11、i3、bspwm、Openbox | Wine 原生 X11 桌面采集 (`BitBlt` / `StretchBlt`) + 光标合成 | X11 **XTest** 扩展（免 root、无需配置 `/dev/uinput` 权限） |
| **Wayland (Portal)** | **KDE Plasma**、**GNOME**、Cosmic、Hyprland、Sway | XDG `ScreenCast` Portal → **PipeWire** (`uur-pw-capture`) → 共享内存三缓冲 | XDG `RemoteDesktop` Portal 或 `/dev/uinput` |
| **Wayland (Direct)** | **Niri**、支持 `wlr-screencopy` 的合成器 | `zwlr_screencopy_manager_v1` (`uur-wl-capture`) | `/dev/uinput` |

### 极简底层二进制依赖（ABI 边界说明）

- `/usr/bin/uur` (`ELF64 PIE`, Full RELRO, NX)：仅动态链接 `libc.so.6`、`libm.so.6` 与 `libgcc_s.so.1`（`GLIBC_2.39+`）。X11 协议（`x11rb`）与 D-Bus 协议（`zbus`）均由纯 Rust 实现，不依赖特定桌面的 GUI 动态库。
- `/usr/lib/uur/uur-pw-capture` (`ELF64 PIE`)：仅链接 `libpipewire-0.3.so.0` 与 `libc.so.6`（仅在 Wayland Portal 模式下按需调用）。
- `/usr/lib/uur/uur-wl-capture` (`ELF64 PIE`)：仅链接 `libwayland-client.so.0` 与 `libc.so.6`（仅在 Wayland 直连模式下按需调用）。
- `/usr/lib/uur/hook/*` (`PE64 x86_64`)：7 个自包含的 Win64 运行时钩子与代理组件，运行于受管 Wine 前缀内部。

---

## 📦 安装指南

请直接从本仓库的 [**GitHub Releases**](https://github.com/truemanshum/uu-remote-linux/releases/latest) 下载最新发布包：

- **`uu-remote-linux_0.2.0_amd64.deb`** — 适用于 **Ubuntu 24.04+、Linux Mint 22.x、Debian 13+、Pop!_OS、Zorin OS、Deepin** 等 Debian/Ubuntu 系发行版
- **`uu-remote-linux-0.2.0-linux-x86_64.tar.zst`** — 适用于 **Arch Linux、Fedora、openSUSE、Gentoo、Void Linux** 等所有 Linux `x86_64` 发行版的通用压缩包
- **`SHA256SUMS`** — 产物 SHA-256 校验清单

### 方式一：Ubuntu / Debian / Linux Mint 安装 (`.deb`)

由于网易官方安装器外壳为 32 位 PE32 程序（安装后的主程序与服务均为 64 位 PE32+），首次安装前请确保已开启 `i386` 多架构支持：

```bash
# 1. 开启 32 位架构支持（若已开启可跳过）
sudo dpkg --add-architecture i386
sudo apt update

# 2. 校验安装包完整性并安装
sha256sum --check uu-remote-linux_0.2.0_amd64.deb.sha256
sudo apt install ./uu-remote-linux_0.2.0_amd64.deb
```

> **无缝升级说明**：`uu-remote-linux` 已声明 `Provides/Conflicts/Replaces: uur, uu-remote-mint`。如果系统中之前安装过 `uur` 或 `uu-remote-mint`，执行上述 `sudo apt install` 会自动完成无缝替换升级，并完整保留你已登录的账号与设备配置。同时完美兼容系统自带 `wine` 以及官方 `winehq-staging` / `winehq-stable` / `winehq-devel`。

### 方式二：Arch Linux / Fedora / openSUSE 等发行版安装（通用 `.tar.zst`）

```bash
sha256sum --check uu-remote-linux-0.2.0-linux-x86_64.tar.zst.sha256
sudo tar --zstd -C / -xf ./uu-remote-linux-0.2.0-linux-x86_64.tar.zst
```

解压后请确保已通过系统包管理器安装 `wine`（支持 32/64 位）、`pipewire`（Wayland 会话需要）、`curl` 与中文字体（如 `noto-fonts-cjk`）。

### 方式三：从源码编译构建

```bash
# 一键构建 .deb 与 .tar.zst 发布包（产物位于 ./dist/）
bash packaging/build-release.sh

# 或者直接编译并安装到当前用户的 ~/.local 目录
cargo build --release --locked
./hook/build.sh
./capture/build.sh
./scripts/install-user.sh
```

---

## ⚡ 首次运行（仅需 3 步）

安装完成后，在桌面终端中按顺序执行以下 3 条命令：

```bash
# 1. 检查当前 Wine、图形会话（X11/Wayland）、音频与输入桥接状态
uur doctor

# 2. 接受网易官方用户协议，自动下载官方客户端并初始化受管 Wine 前缀
uur setup --accept-eula

# 3. 启动画面采集、输入桥接、后台服务与网易 UU 远程主界面
uur run
```

初始化完成后，你也可以直接从系统桌面左下角的**应用程序菜单**点击 **UU Remote** 启动，或开启开机自启：

```bash
uur autostart enable
```

---

## 🧭 常用命令速查

```bash
uur run                          # 启动或唤起网易 UU 远程会话
uur doctor                       # 运行只读环境诊断（检查 Wine、XTest/Portal、钩子与服务状态）
uur stop                         # 停止 UU 远程后台服务与受管 Wine 前缀
uur autostart enable             # 开启桌面登录后自动启动
uur autostart disable            # 关闭桌面登录自动启动
uur upstream check               # 检查网易官方是否有新版本客户端发布
uur wol status                   # 查看当前有线网卡的远程唤醒 (Wake-on-LAN) 状态
uur display status               # 查看当前显示器/窗口采集状态
uur uninstall --purge --dry-run  # 预览彻底清理当前用户的 Wine 前缀与缓存目录
uur uninstall --purge            # 彻底清理当前用户的 Wine 前缀、日志与安装包缓存
```

---

## 🛠️ 故障排查

1. **首选诊断命令**：
   ```bash
   uur doctor
   ```
   确保所有检查项均为 `ok`（在 X11 会话下 ` Wayland runtime` 显示 `warn` 属于正常现象，因为 X11 直接使用原生采集与 `xtest` 注入）。
2. **查看会话运行日志**：
   ```bash
   cat "${XDG_STATE_HOME:-$HOME/.local/state}/uur/session.log"
   ```
3. **验证防鼠标锁死钩子状态**：
   在远控连接期间，可查看受管 Wine 前缀下的钩子状态文件：
   ```bash
   cat ~/.local/share/uur/wine/drive_c/uur-capture-status-*.txt
   ```
   其中 `iat=94` 表示 `SendInput`、`GetCursorInfo`、`GetCursorPos`、`BitBlt`、`StretchBlt` 等导入表已成功挂钩，`cursor_info=` 会实时统计远控端查询光标状态的次数。

---

## 📚 详细文档导航

- [**全平台安装与配置指南**](INSTALL.md)
- [**版本更新日志 (CHANGELOG)**](CHANGELOG.md)
- [**系统架构设计**](docs/architecture.md)
- [**跨平台支持说明**](docs/platform-support.md)
- [**构建溯源与 ABI 边界说明**](docs/provenance.md)
- [**画面采集帧协议**](docs/capture-protocol.md)
- [**Linux 原生服务适配器**](docs/native-services.md)

---

## 🙏 致谢与参考项目

本项目面向全系 Linux 桌面环境进行了系统级兼容层设计与工程重构，特此感谢以下开源项目的研究与贡献：

- 上游基础架构参考：[`panxuc/uur`](https://github.com/panxuc/uur) (`v0.1.5`, MIT License)
- 早期 Linux Wine 运行时与桥接方案探索：
  - [`GuoWQ222/uu-remote-for-linux`](https://github.com/GuoWQ222/uu-remote-for-linux)
  - [`lachlanchen/uu-remote-ubuntu-bridge`](https://github.com/lachlanchen/uu-remote-ubuntu-bridge)

**免责声明**：网易 UU 远程（GameViewer）是网易公司（NetEase, Inc.）的专有商业软件。`uu-remote-linux` 是独立的第三方开源系统兼容层项目，与网易公司不存在任何隶属或官方背书关系。
