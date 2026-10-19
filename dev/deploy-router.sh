#!/usr/bin/env bash
# Swaps a fresh build of the daemon and web app into a router that already has the coWRT package.
# Usage: dev/deploy-router.sh root@10.0.0.1 [aarch64|x86_64]
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
host=${1:?Usage: dev/deploy-router.sh user@router [aarch64|x86_64]}
target=$([ "${2:-aarch64}" = x86_64 ] && echo x86_64-unknown-linux-musl || echo aarch64-unknown-linux-musl)
www=$(mktemp)
trap 'rm -f "$www"' EXIT

(cd "$root/daemon" && cargo zigbuild --release --quiet --target "$target")
(cd "$root/web" && bun run build >/dev/null)
tar -C "$root/web/build" -czf "$www" .

scp -O -q "$root/daemon/target/$target/release/cowrtd" "$host:/tmp/cowrtd.new"
scp -O -q "$www" "$host:/tmp/cowrt-www.tar.gz"
ssh "$host" '/etc/init.d/cowrtd stop
mv /tmp/cowrtd.new /usr/sbin/cowrtd && chmod 755 /usr/sbin/cowrtd
rm -rf /usr/share/cowrt/www && mkdir -p /usr/share/cowrt/www
tar -xzf /tmp/cowrt-www.tar.gz -C /usr/share/cowrt/www && rm /tmp/cowrt-www.tar.gz
/etc/init.d/cowrtd start'
echo "Deployed to $host"
