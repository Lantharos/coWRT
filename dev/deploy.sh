#!/usr/bin/env bash
# Builds coWRT and installs it into the development VM (dev/vm/run.sh).
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
vm="$root/dev/vm/ssh.sh"
target=x86_64-unknown-linux-musl

(cd "$root/daemon" && cargo zigbuild --release --quiet --target "$target")
"$vm" '/etc/init.d/cowrtd stop 2>/dev/null; mkdir -p /usr/share/cowrt/www'
"$vm" 'cat > /usr/sbin/cowrtd && chmod 755 /usr/sbin/cowrtd' <"$root/daemon/target/$target/release/cowrtd"
"$vm" 'cat > /etc/init.d/cowrtd && chmod 755 /etc/init.d/cowrtd' <"$root/packaging/openwrt/files/cowrtd.init"
"$vm" '[ -f /etc/config/cowrt ] || cat > /etc/config/cowrt' <"$root/packaging/openwrt/files/cowrt.config"

if [ -d "$root/web/build" ]; then
	tar -C "$root/web/build" -cf - . | "$vm" 'rm -rf /usr/share/cowrt/www/* && tar -C /usr/share/cowrt/www -xf -'
fi

"$vm" 'if uci -q get uhttpd.main.listen_http | grep -q ":80$"; then
	uci set uhttpd.main.listen_http="0.0.0.0:8080 [::]:8080"
	uci set uhttpd.main.listen_https="0.0.0.0:8443 [::]:8443"
	uci commit uhttpd
	/etc/init.d/uhttpd restart
fi
/etc/init.d/cowrtd enable
/etc/init.d/cowrtd start'
echo "coWRT is running at http://127.0.0.1:10080"
