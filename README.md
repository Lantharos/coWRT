# coWRT

A modern interface for OpenWrt routers. coWRT replaces LuCI with a fast, friendly web app for everything you do with a router, from seeing who's online to running a VPN, without ever touching a config file.

## What it does

- **Overview**: whether you're online, how many devices are connected, today's usage and live speed.
- **Devices**: every device with its network, signal history, live speed, 30 days of usage and the sites it looked up today. Rename it, reserve its address, wake it up, pause its internet, set quiet times, cap its speed or give it priority. Group devices under people to pause them together, give them a bedtime, share one speed limit across their devices or set their priority.
- **Wi-Fi**: networks spanning every band, which network their devices join, QR codes for joining, guest networks that turn themselves off, fast roaming and band steering, and per-radio channel, width and power with a scan for the quietest channel.
- **Networks**: groups of devices with their own IPv4 range and IPv6 prefix, ports, Wi-Fi and rules. Plus the internet connection (automatic, fixed or PPPoE, with an optional VLAN tag) with a backup like a tethered phone, IPv6 with prefix delegation, a speed test that also measures lag under load, a smart queue against bufferbloat that can set itself from the speed test, acceleration, local names like `nas.lan` and static routes.
- **Access**: a reachability test, port forwards that also open over IPv6, automatic port forwarding (UPnP and NAT-PMP), dynamic DNS on Cloudflare, a WireGuard VPN with QR codes, and routing whole networks through a VPN provider with a kill switch.
- **Security**: ad, tracker and threat blocking per network with a running count of what was blocked, an allow list, and encrypted DNS.
- **System**: health charts, router name and time zone, password, backups, firmware, restarts, logs and coWRT updates.

Press ⌘K (or Ctrl+K, or /) anywhere to jump to a device, person, network or setting. On a fresh router, coWRT starts with a short setup: a password, the Wi-Fi name and the time zone. It can be added to a phone's home screen like an app.

## How it fits together

| Path | What lives there |
| --- | --- |
| `daemon/` | `cowrtd`, a small Rust service that runs on the router. It talks to OpenWrt over ubus, serves the web app, and streams live stats over a WebSocket. |
| `web/` | The web app, built with SvelteKit and Tailwind and shipped to the router as static files. |
| `packaging/` | The OpenWrt package definition and the script that builds it. |
| `worker/` | A Cloudflare Worker that tells a router its public address and checks whether its ports are reachable from outside. It only ever probes the address that asks. |
| `dev/` | Scripts for running a development router in a VM and deploying to it. |

`cowrtd` signs you in with the router's root password and uses OpenWrt's own configuration system, so anything you change in coWRT shows up in UCI as usual, and the other way around.

## Networks

coWRT thinks in networks rather than bridges, devices and interfaces. A network is a group of devices with its own address range, and you decide whether it can reach the internet and whether it can reach your other networks. Ports and Wi-Fi networks simply say which network they put devices on.

Under the hood every network is a VLAN on one VLAN-aware bridge, with a matching interface, DHCP pool and firewall zone. The first time `cowrtd` starts it converts the stock OpenWrt layout to this model: the existing LAN becomes "Home" on VLAN 1, and if it still uses the stock 192.168.1.1 it moves to 10.0.0.1 (or 10.10.0.1 when the upstream connection already uses 10.0.0.x). Address reservations move along with it. New networks get 10.0.N.1/24, where N is their VLAN number.

Changes that could cut you off, like editing a network, a port or the internet connection, are rolled back automatically after a minute unless your browser checks back in with the router.

## Device rules and usage

coWRT keeps its own nftables tables next to the stock firewall. Pausing a device or giving it quiet times adds rules on the forward path, speed limits are enforced on each network's interface, and priority marks connections so the smart queue (cake on the WAN) can serve them first or last. Per-device speeds and daily totals come from the Wi-Fi driver and switch port counters, with connection tracking for devices that share a port, so they're counted even when acceleration is on, but limits and priority only apply to traffic the router's processor sees.

## Backup connections

The router can keep a second connection as a backup, like a phone plugged into its USB port with tethering on. Every few seconds coWRT checks each connection for real internet, not just a link light. When the main line stops answering it moves traffic to the backup, and once the main line has been steady for a minute it moves back. You can also pin either connection by hand. Under the hood the active connection's route lives in its own routing table, so switching never touches the interfaces themselves.

Setting up a phone installs the USB tethering drivers for Android and iPhone. The router notices when a phone is plugged in and uses it whatever name its network device gets. If the phone offers IPv6, that moves over too: phones share a single /64 rather than delegating a prefix, so while the backup is in use the router translates your devices' IPv6 addresses to its own address on the phone, and stops as soon as it moves back. coWRT replaces mwan3 for this, so it turns mwan3 off when it takes over.

## IPv6

IPv6 works like IPv4 throughout. The router asks your provider for a prefix (you can pick the size) and hands each network its own /64 from it, numbered after the network's VLAN, so Guest on VLAN 10 gets `…:a::/64`. Every network also gets a local prefix that stays the same if the provider's prefix changes. Port forwards open the same ports over IPv6: coWRT learns the stable part of the device's IPv6 addresses and allows traffic to them whatever the current prefix is.

## Site activity

Each network's DNS lookups go through a small resolver that coWRT runs next to the main one, with or without block lists. Those resolvers log lookups to memory, and coWRT counts them per device: the sites looked up today, what was blocked, and a 24-hour graph of blocked lookups. Nothing is written to flash, the counts reset every night, and it can be switched off in Security. Daily traffic totals per device are kept for 30 days in `/etc/cowrt/usage.json`.

## Speed test

The speed test runs on the router itself against Cloudflare's speed test servers: idle latency first, then eight seconds of download and upload over six connections each, while it keeps measuring latency to see how much the line lags under load. The smart queue is paused during the test so it measures the real line. With "Set speeds from the speed test" on, the smart queue uses 90% of the measured speeds. The last 30 results are kept in `/etc/cowrt/speedtests.json`.

## Extras installed on demand

Some features pull in an OpenWrt package the first time you switch them on: UPnP (`miniupnpd-nftables`), the VPN (`kmod-wireguard`, `wireguard-tools`), encrypted DNS (`https-dns-proxy`) and band steering (`usteer`). The router needs to be online for that. Fast roaming uses 802.11r everywhere, and adds 802.11k and 802.11v when the full `wpad` package is installed instead of the basic one.

## Releases

Pushing a tag like `v0.2.0` that matches the version in `daemon/Cargo.toml` builds both packages on GitHub Actions and publishes them with a `SHA256SUMS` file. Routers check the latest release from the System page and verify the download before installing it.

## Development

You'll need Rust with [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild), Bun, and QEMU with KVM.

Start the development router. The first run downloads and verifies the OpenWrt image:

```bash
dev/vm/run.sh -daemonize
```

The VM's root password is whatever you set over the serial console (`.vm/console.sock`). SSH in with `dev/vm/ssh.sh` once your key is in `/etc/dropbear/authorized_keys`. The VM's LAN ports have nothing attached, so everything reaches it through its WAN port: SSH on 127.0.0.1:10022 and the web interface on 127.0.0.1:18080.

Build the package and install it into the VM:

```bash
packaging/build.sh x86_64
dev/vm/ssh.sh 'cat > /tmp/cowrt.apk && apk add --allow-untrusted /tmp/cowrt.apk' < dist/cowrt-0.1.0-x86_64.apk
```

After that, `dev/deploy.sh` swaps in a fresh build of the daemon and web app in a few seconds.

To try a build on a real router that already has the package installed, `dev/deploy-router.sh root@10.0.0.1` does the same over SSH (add `x86_64` for an x86 router).

The VM's LAN side has three ports (eth0, eth2 and eth3) and its WAN is eth1. For frontend work, run the Vite dev server instead. It proxies `/api` to the VM:

```bash
bun run --cwd web dev
```

Point it at a different router with `COWRT_ROUTER=http://10.0.0.1`.

### Simulated Wi-Fi clients

The VM uses `mac80211_hwsim` radios. `dev/vm/clients.sh` runs inside the VM at boot, moves two spare radios into their own network namespaces, and connects them to the "Home" network, so the device list has real clients to show. Run commands as one of them with `ip netns exec Pixel-9 …`.

### The Worker

```bash
cd worker && bun install && bunx wrangler deploy
```

## Building for a router

```bash
packaging/build.sh
```

This builds the web app, cross-compiles `cowrtd`, and packages both with the official OpenWrt SDK containers (Podman), producing `dist/cowrt-<version>-x86_64.apk` and `dist/cowrt-<version>-aarch64_cortex-a53.apk`. The ARM package fits the ASUS TUF-AX6000 and other MediaTek Filogic routers.

Copy the package to the router and install it:

```bash
apk add --allow-untrusted /tmp/cowrt-0.1.0-aarch64_cortex-a53.apk
```

coWRT takes over port 80 and moves the stock web server (and LuCI with it) to port 8080. Removing the package with `apk del cowrt` moves it back.
