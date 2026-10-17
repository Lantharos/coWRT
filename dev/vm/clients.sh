#!/bin/sh
# Runs inside the VM: turns spare hwsim radios into Wi-Fi clients, each in its own network namespace.

cat >/tmp/client-dhcp.sh <<-'EOF'
	#!/bin/sh
	case "$1" in
	bound | renew)
		ip addr flush dev "$interface"
		ip addr add "$ip/$subnet" dev "$interface"
		;;
	deconfig)
		ip addr flush dev "$interface"
		;;
	esac
EOF
chmod +x /tmp/client-dhcp.sh

join() {
	phy=$1 name=$2 freq=$3
	ip netns add "$name" 2>/dev/null || return 0
	iw phy "$phy" set netns name "$name"
	ip netns exec "$name" ip link set lo up
	dev=wlan0
	ip netns exec "$name" iw phy "$phy" interface add "$dev" type managed
	cat >"/tmp/$name.conf" <<-EOF
		network={
			ssid="Home"
			psk="password123"
			key_mgmt=SAE WPA-PSK
			ieee80211w=1
			scan_freq=$freq
		}
	EOF
	ip netns exec "$name" wpa_supplicant -B -i "$dev" -c "/tmp/$name.conf"
	ip netns exec "$name" udhcpc -b -i "$dev" -x "hostname:$name" -s /tmp/client-dhcp.sh
}

join phy2 Pixel-9 2437
join phy3 MacBook-Air 5180
