# coWRT

A modern interface for OpenWrt routers. coWRT replaces LuCI with a fast, friendly web app for everything you do with a router, from seeing who's online to running a VPN, without ever touching a config file.

## What it does

- **Overview**: whether you're online, how many devices are connected, today's usage and live speed.
- **Devices**: every device with its network, signal and live speed. Rename it, reserve its address, pause its internet, set quiet times, cap its speed or give it priority.
- **Wi-Fi**: networks spanning both bands, which network their devices join, and per-radio channel, width and power.
- **Networks**: groups of devices with their own address range, ports, Wi-Fi and rules. Plus the internet connection (automatic, fixed or PPPoE, with an optional VLAN tag), a smart queue against bufferbloat, and acceleration.
- **Access**: a reachability test, port forwards, automatic port forwarding (UPnP and NAT-PMP), dynamic DNS on Cloudflare, a WireGuard VPN with QR codes, and routing whole networks through a VPN provider with a kill switch.
- **Security**: ad, tracker and threat blocking per network, an allow list, and encrypted DNS.
- **System**: health charts, router name and time zone, password, backups, firmware, restarts, logs and coWRT updates.

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

coWRT keeps its own nftables tables next to the stock firewall. Pausing a device or giving it quiet times adds rules on the forward path, speed limits are enforced on each network's interface, and priority marks connections so the smart queue (cake on the WAN) can serve them first or last. Per-device speeds and daily totals come from connection tracking, so they're counted even when acceleration is on, but limits and priority only apply to traffic the router's processor sees.

## Extras installed on demand

Some features pull in an OpenWrt package the first time you switch them on: UPnP (`miniupnpd-nftables`), the VPN (`kmod-wireguard`, `wireguard-tools`) and encrypted DNS (`https-dns-proxy`). The router needs to be online for that.

## Releases

Pushing a tag like `v0.2.0` that matches the version in `daemon/Cargo.toml` builds both packages on GitHub Actions and publishes them with a `SHA256SUMS` file. Routers check the latest release from the System page and verify the download before installing it.

## Development

You'll need Rust with [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild), Bun, and QEMU with KVM.

Start the development router. The first run downloads and verifies the OpenWrt image:

```bash
dev/vm/run.sh -daemonize
```

The VM's root password is whatever you set over the serial console (`.vm/console.sock`). SSH in with `dev/vm/ssh.sh` once your key is in `/etc/dropbear/authorized_keys`. The VM's LAN ports have nothing attached, so everything reaches it through its WAN port: SSH on 127.0.0.1:10022 and the web interface on 127.0.0.1:10080.

Build the package and install it into the VM:

```bash
packaging/build.sh x86_64
dev/vm/ssh.sh 'cat > /tmp/cowrt.apk && apk add --allow-untrusted /tmp/cowrt.apk' < dist/cowrt-0.1.0-x86_64.apk
```

After that, `dev/deploy.sh` swaps in a fresh build of the daemon and web app in a few seconds.

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
