#!/usr/bin/env bash
set -euo pipefail

project_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
package_name=uu-remote-linux
package_version=0.2.0
architecture=amd64
work_root=${UUR_WORK_ROOT:-$project_root/.work/linux-pkg}
output_dir=${UUR_OUTPUT_DIR:-$project_root/dist}
mkdir -p "$work_root"
owned_work=$(mktemp -d "$work_root/package.XXXXXX")
trap 'rm -rf -- "$owned_work"' EXIT
package_root="$owned_work/root"
portable_root="$owned_work/portable"
debian_dir="$package_root/DEBIAN"
artifact="$output_dir/${package_name}_${package_version}_${architecture}.deb"
portable_artifact="$output_dir/${package_name}-${package_version}-linux-x86_64.tar.zst"

source_date_epoch=${SOURCE_DATE_EPOCH:-$(git -C "$project_root" log -1 --format=%ct)}
SOURCE_DATE_EPOCH=$source_date_epoch
export SOURCE_DATE_EPOCH

mkdir -p "$debian_dir" "$portable_root" "$output_dir"

cargo build --manifest-path "$project_root/Cargo.toml" --release --locked
"$project_root/hook/build.sh"
"$project_root/capture/build.sh"
cargo_target_dir=${CARGO_TARGET_DIR:-$project_root/target}
if [[ $cargo_target_dir != /* ]]; then
    cargo_target_dir="$(pwd -P)/$cargo_target_dir"
fi
UUR_BINARY="$cargo_target_dir/release/uur" UUR_STAGE_UINPUT=0 \
    "$project_root/packaging/stage.sh" "$package_root"
UUR_BINARY="$cargo_target_dir/release/uur" UUR_STAGE_UINPUT=1 \
    "$project_root/packaging/stage.sh" "$portable_root"

cat >"$debian_dir/control" <<EOF
Package: $package_name
Version: $package_version
Section: net
Priority: optional
Architecture: $architecture
Maintainer: truemanshum <https://github.com/truemanshum/uu-remote-linux>
Homepage: https://github.com/truemanshum/uu-remote-linux
Depends: libc6, libgcc-s1, libglib2.0-bin, libx11-6, libxtst6, libpipewire-0.3-0t64 | libpipewire-0.3-0, libwayland-client0, wine | winehq-staging | winehq-stable | winehq-devel, wine64 | winehq-staging | winehq-stable | winehq-devel, wine32 | wine32:i386 | winehq-staging | winehq-stable | winehq-devel, fonts-noto-cjk, curl, coreutils
Recommends: ethtool, network-manager, pulseaudio-utils
Provides: uur (= $package_version), uu-remote-mint (= $package_version)
Conflicts: uur, uu-remote-mint
Replaces: uur, uu-remote-mint
Description: Universal Linux compatibility layer for NetEase UU Remote (X11 & Wayland)
 Provides the native Linux supervisor, X11/Wayland screen capture bridges,
 XTest/Portal/uinput input adapters, and Wine PE64 runtime hooks (including
 cursor lock prevention and multi-version launcher auto-healing). The
 proprietary NetEase client is not bundled and is downloaded only after EULA
 acceptance.
EOF

install -Dm0755 "$project_root/packaging/package-postinstall.sh" \
    "$debian_dir/postinst"
(
    cd "$package_root"
    find usr -type f -print0 | sort -z | xargs -0 md5sum >DEBIAN/md5sums
)
find "$package_root" "$portable_root" -print0 | xargs -0 touch --no-dereference --date="@$source_date_epoch"
chmod 0755 "$debian_dir" "$debian_dir/postinst"
chmod 0644 "$debian_dir/control" "$debian_dir/md5sums"

rm -f -- "$artifact" "$artifact.sha256" "$portable_artifact" "$portable_artifact.sha256" "$output_dir/SHA256SUMS"
dpkg-deb --root-owner-group --build -Zxz -z9 "$package_root" "$artifact"
tar --sort=name --mtime="@$source_date_epoch" --owner=0 --group=0 --numeric-owner \
    --zstd -C "$portable_root" -cf "$portable_artifact" usr
(
    cd "$output_dir"
    sha256sum "${artifact##*/}" >"${artifact##*/}.sha256"
    sha256sum "${portable_artifact##*/}" >"${portable_artifact##*/}.sha256"
    sha256sum "${artifact##*/}" "${portable_artifact##*/}" >SHA256SUMS
    sha256sum --check --strict "${artifact##*/}.sha256"
    sha256sum --check --strict "${portable_artifact##*/}.sha256"
    sha256sum --check --strict SHA256SUMS
)
printf 'built: %s\nbuilt: %s\nchecksums: %s/SHA256SUMS\n' "$artifact" "$portable_artifact" "$output_dir"
