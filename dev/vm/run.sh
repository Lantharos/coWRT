#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
dir="$root/.vm"
version=25.12.5
base="https://downloads.openwrt.org/releases/$version/targets/x86/64"
archive="openwrt-$version-x86-64-generic-squashfs-combined.img.gz"
image="$dir/openwrt-$version.img"

mkdir -p "$dir"

if [ ! -f "$image" ]; then
	curl -fsSL -o "$dir/$archive" "$base/$archive"
	(cd "$dir" && curl -fsSL "$base/sha256sums" | grep " \*\?$archive\$" | sha256sum -c --quiet)
	gunzip -c "$dir/$archive" >"$image" 2>/dev/null || true
	rm "$dir/$archive"
	truncate -s 1G "$image"
fi

exec qemu-system-x86_64 \
	-enable-kvm -cpu host -smp 2 -m 512 \
	-drive "file=$image,format=raw,if=virtio" \
	-device virtio-net-pci,mac=52:54:00:12:34:56 \
	-netdev "user,id=wan,hostfwd=tcp:127.0.0.1:10022-:22,hostfwd=tcp:127.0.0.1:18080-:80" \
	-device virtio-net-pci,netdev=wan,mac=52:54:00:12:34:57 \
	-device virtio-net-pci,mac=52:54:00:12:34:58 \
	-device virtio-net-pci,mac=52:54:00:12:34:59 \
	-display none \
	-serial "unix:$dir/console.sock,server,nowait" \
	-pidfile "$dir/qemu.pid" \
	"$@"
