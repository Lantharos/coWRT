# coWRT

A modern interface for OpenWrt routers. coWRT replaces LuCI with a fast, friendly web app for the things you actually do with a router: seeing who's online, managing Wi-Fi, reserving addresses, and keeping an eye on your connection.

## How it fits together

| Path | What lives there |
| --- | --- |
| `daemon/` | `cowrtd`, a small Rust service that runs on the router. It talks to OpenWrt over ubus, serves the web app, and streams live stats over a WebSocket. |
| `web/` | The web app, built with SvelteKit and Tailwind and shipped to the router as static files. |
| `packaging/openwrt/` | The init script and default config installed on the router. |
| `dev/` | Scripts for running a development router in a VM and deploying to it. |

`cowrtd` signs you in with the router's root password and uses OpenWrt's own configuration system, so anything you change in coWRT shows up in UCI as usual, and the other way around.

## Networks

coWRT thinks in networks rather than bridges, devices and interfaces. A network is a group of devices with its own address range, and you decide whether it can reach the internet and whether it can reach your other networks. Ports and Wi-Fi networks simply say which network they put devices on.

Under the hood every network is a VLAN on one VLAN-aware bridge, with a matching interface, DHCP pool and firewall zone. The first time `cowrtd` starts it converts the stock OpenWrt layout to this model: the existing LAN becomes "Home" on VLAN 1, and if it still uses the stock 192.168.1.1 it moves to 10.0.0.1 (or 10.10.0.1 when the upstream connection already uses 10.0.0.x). Address reservations move along with it. New networks get 10.0.N.1/24, where N is their VLAN number.

Changes that could cut you off, like editing a network or a port, are rolled back automatically after a minute unless your browser checks back in with the router.

## Development

You'll need Rust with [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild), Bun, and QEMU with KVM.

Start the development router. The first run downloads and verifies the OpenWrt image:

```bash
dev/vm/run.sh -daemonize
```

The VM's root password is whatever you set over the serial console (`.vm/console.sock`). SSH in with `dev/vm/ssh.sh` once your key is in `/etc/dropbear/authorized_keys`.

Build coWRT and install it into the VM:

```bash
dev/deploy.sh
```

The VM's LAN side has three ports (eth0, eth2 and eth3) and its WAN is eth1. The router UI is then served at http://127.0.0.1:10080. For frontend work, run the Vite dev server instead. It proxies `/api` to the VM:

```bash
bun run --cwd web dev
```

Point it at a different router with `COWRT_ROUTER=http://192.168.1.1`.

### Simulated Wi-Fi clients

The VM uses `mac80211_hwsim` radios. `dev/vm/clients.sh` runs inside the VM at boot, moves two spare radios into their own network namespaces, and connects them to the "Home" network, so the device list has real clients to show.

## Building for a router

The ASUS TUF-AX6000 (MediaTek Filogic) and most current routers use 64-bit ARM:

```bash
cd daemon && cargo zigbuild --release --target aarch64-unknown-linux-musl
cd web && bun run build
```

Copy `daemon/target/aarch64-unknown-linux-musl/release/cowrtd` to `/usr/sbin/cowrtd`, `web/build` to `/usr/share/cowrt/www`, and the files in `packaging/openwrt/files` to `/etc/init.d/cowrtd` and `/etc/config/cowrt`. coWRT listens on port 80 by default, so move uhttpd (and LuCI with it) to another port first.
