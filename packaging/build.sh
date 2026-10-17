#!/usr/bin/env bash
# Builds coWRT packages into dist/ using the official OpenWrt SDK containers.
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

sdk_image() {
	case $1 in
	x86_64) echo "ghcr.io/openwrt/sdk:x86-64-v$openwrt" ;;
	aarch64_cortex-a53) echo "ghcr.io/openwrt/sdk:mediatek-filogic-v$openwrt" ;;
	*) echo "no SDK known for $1" >&2 && exit 1 ;;
	esac
}

(cd "$root/web" && bun install --frozen-lockfile --silent && bun run build >/dev/null)
mkdir -p "$root/dist"

for arch in "${arches[@]}"; do
	target=$(rust_target "$arch")
	(cd "$root/daemon" && cargo zigbuild --release --quiet --target "$target")

	stage=$(mktemp -d)
	trap 'rm -rf "$stage"' EXIT
	cp "$root/daemon/target/$target/release/cowrtd" "$stage/"
	cp -r "$root/web/build" "$stage/www"

	podman run --rm --userns=keep-id \
		-v "$root/packaging/openwrt/cowrt:/builder/package/cowrt:ro,z" \
		-v "$stage:/stage:ro,z" \
		-v "$root/dist:/out:z" \
		"$(sdk_image "$arch")" \
		bash -c "make defconfig >/dev/null && make package/cowrt/compile COWRT_STAGE=/stage COWRT_VERSION=$version >/dev/null && find bin/packages -name 'cowrt-*.apk' -exec cp {} /out/cowrt-$version-$arch.apk \;"
	rm -rf "$stage"
	trap - EXIT
done

ls -1 "$root/dist"/cowrt-"$version"-*.apk
