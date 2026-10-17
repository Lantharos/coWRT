#!/usr/bin/env bash
# Builds coWRT packages into dist/.
# Usage: packaging/build.sh [x86_64] [aarch64_cortex-a53]   (default: both)
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
openwrt=25.12.5
version=$(grep -m1 '^version' "$root/daemon/Cargo.toml" | cut -d'"' -f2)
arches=("$@")
[ ${#arches[@]} -gt 0 ] || arches=(x86_64 aarch64_cortex-a53)

rust_target() {
	case $1 in
	x86_64) echo x86_64-unknown-linux-musl ;;
	aarch64_*) echo aarch64-unknown-linux-musl ;;
	*) echo "unsupported architecture $1" >&2 && exit 1 ;;
	esac
}

sdk_target() {
	case $1 in
	x86_64) echo x86/64 ;;
	aarch64_cortex-a53) echo mediatek/filogic ;;
	*) echo "no SDK known for $1" >&2 && exit 1 ;;
	esac
}

fetch_sdk() {
	local arch=$1 dir="$root/.sdk/$1"
	[ -f "$dir/rules.mk" ] && return
	local base="https://downloads.openwrt.org/releases/$openwrt/targets/$(sdk_target "$arch")"
	local archive
	archive=$(curl -fsSL "$base/sha256sums" | grep -oE 'openwrt-sdk-[^ ]+\.tar\.zst$' | tr -d '*')
	mkdir -p "$dir"
	curl -fsSL -o "$root/.sdk/$archive" "$base/$archive"
	(cd "$root/.sdk" && curl -fsSL "$base/sha256sums" | grep " \*\?$archive\$" | sha256sum -c --quiet)
	tar --zstd -xf "$root/.sdk/$archive" -C "$dir" --strip-components=1
	rm "$root/.sdk/$archive"
}

(cd "$root/web" && bun install --frozen-lockfile --silent && bun run build >/dev/null)
mkdir -p "$root/dist"

for arch in "${arches[@]}"; do
	target=$(rust_target "$arch")
	(cd "$root/daemon" && cargo zigbuild --release --quiet --target "$target")

	stage="$root/.sdk/stage/$arch"
	rm -rf "$stage" && mkdir -p "$stage"
	cp "$root/daemon/target/$target/release/cowrtd" "$stage/"
	cp -r "$root/web/build" "$stage/www"

	fetch_sdk "$arch"
	sdk="$root/.sdk/$arch"
	ln -sfn "$root/packaging/openwrt/cowrt" "$sdk/package/cowrt"
	[ -f "$sdk/.config" ] || make -C "$sdk" defconfig >/dev/null
	make -C "$sdk" package/cowrt/clean package/cowrt/compile COWRT_STAGE="$stage" COWRT_VERSION="$version" >/dev/null
	find "$sdk/bin/packages" -name "cowrt-$version-r*.apk" -exec cp {} "$root/dist/" \;
done

ls -1 "$root/dist"/cowrt-"$version"-*.apk
