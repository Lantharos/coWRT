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
	-netdev socket,id=lan,mcast=230.0.0.1:10101 \
	-device virtio-net-pci,netdev=lan \
	-netdev "user,id=wan,hostfwd=tcp:127.0.0.1:10022-:22,hostfwd=tcp:127.0.0.1:10080-:80" \
	-device virtio-net-pci,netdev=wan \
	-netdev socket,id=port2,mcast=230.0.0.1:10102 \
	-device virtio-net-pci,netdev=port2 \
	-netdev socket,id=port3,mcast=230.0.0.1:10103 \
	-device virtio-net-pci,netdev=port3 \
	-display none \
	-serial "unix:$dir/console.sock,server,nowait" \
	-pidfile "$dir/qemu.pid" \
	"$@"
