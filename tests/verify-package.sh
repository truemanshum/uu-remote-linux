#!/usr/bin/env bash
set -euo pipefail

deb=${1:?usage: tests/verify-package.sh PACKAGE.deb}
temporary=$(mktemp -d "${TMPDIR:-/tmp}/uu-remote-linux-package.XXXXXX")
trap 'rm -rf -- "$temporary"' EXIT

assert_clean_payload() {
    local root=$1
    local file digest size

    while IFS= read -r -d '' file; do
        size=$(stat --format=%s "$file")
        [[ $size != 89664288 ]] || {
            printf 'proprietary installer-sized payload found: %s\n' "$file" >&2
            return 1
        }
        digest=$(sha256sum "$file" | cut -d' ' -f1)
        [[ $digest != d3a7776f6fb00b196e2fcbf8acba06fd52b19437b0da80fec0ae838ee362f06f ]] || {
            printf 'known NetEase installer found: %s\n' "$file" >&2
            return 1
        }
    done < <(find "$root" -type f -print0)

    ! find "$root" -type f \( -iname 'uu-remote-setup.exe' -o -iname 'gameviewer.exe' \) -print -quit | grep -q .
}

package_root="$temporary/root"
mkdir -p "$package_root"
dpkg-deb --extract "$deb" "$package_root"

[[ $(dpkg-deb --field "$deb" Package) == uu-remote-linux ]]
[[ $(dpkg-deb --field "$deb" Version) == 0.2.0 ]]
[[ $(dpkg-deb --field "$deb" Architecture) == amd64 ]]
dpkg-deb --field "$deb" Depends | grep -Fq 'libpipewire-0.3-0'
dpkg-deb --field "$deb" Depends | grep -Fq 'libx11-6'
dpkg-deb --field "$deb" Depends | grep -Fq 'libxtst6'
dpkg-deb --field "$deb" Depends | grep -Fq 'wine'
dpkg-deb --field "$deb" Depends | grep -Fq 'wine64'
dpkg-deb --field "$deb" Depends | grep -Fq 'wine32'
[[ $(dpkg-deb --field "$deb" Provides) == 'uur (= 0.2.0), uu-remote-mint (= 0.2.0)' ]]
[[ $(dpkg-deb --field "$deb" Conflicts) == 'uur, uu-remote-mint' ]]
[[ $(dpkg-deb --field "$deb" Replaces) == 'uur, uu-remote-mint' ]]

test -x "$package_root/usr/bin/uur"
test -x "$package_root/usr/lib/uur/uur-pw-capture"
test -x "$package_root/usr/lib/uur/uur-wl-capture"
test -f "$package_root/usr/lib/uur/hook/uur-hook.dll"
test -f "$package_root/usr/share/applications/uur.desktop"
test -f "$package_root/usr/share/icons/hicolor/256x256/apps/uur.png"
test -f "$package_root/usr/share/licenses/uur/LICENSE"
test -f "$package_root/usr/share/licenses/uur/NOTICE"
test ! -e "$package_root/usr/lib/udev/rules.d/60-uur-uinput.rules"
test ! -e "$package_root/usr/lib/modules-load.d/uur.conf"
assert_clean_payload "$package_root"

negative_root="$temporary/negative"
mkdir -p "$negative_root"
truncate --size=89664288 "$negative_root/not-a-mirror.bin"
if assert_clean_payload "$negative_root"; then
    printf 'proprietary payload negative test unexpectedly passed\n' >&2
    exit 1
fi

printf 'Linux package metadata, layout, and payload checks passed\n'
