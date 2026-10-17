#!/usr/bin/env bash
# Swaps a fresh build of cowrtd and the web app into the development VM.
# Install the package once first (packaging/build.sh x86_64, then apk add it).
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
vm="$root/dev/vm/ssh.sh"
target=x86_64-unknown-linux-musl

(cd "$root/daemon" && cargo zigbuild --release --quiet --target "$target")
(cd "$root/web" && bun run build >/dev/null)

"$vm" '/etc/init.d/cowrtd stop'
"$vm" 'cat > /usr/sbin/cowrtd' <"$root/daemon/target/$target/release/cowrtd"
tar -C "$root/web/build" -cf - . | "$vm" 'rm -rf /usr/share/cowrt/www/* && tar -C /usr/share/cowrt/www -xf -'
"$vm" '/etc/init.d/cowrtd start'
echo "coWRT is running at http://127.0.0.1:10080"
