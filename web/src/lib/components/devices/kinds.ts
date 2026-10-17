import {
	Camera,
	Gamepad2,
	Laptop,
	Lightbulb,
	Monitor,
	MonitorSmartphone,
	Printer,
	Server,
	Smartphone,
	Speaker,
	Tablet,
	Tv,
	Watch
} from '@lucide/svelte';
import type { Device } from '#lib/api/types.js';

export const kinds = {
	phone: {
		label: 'Phone',
		icon: Smartphone,
		pattern: /iphone|pixel|galaxy|android|phone|oneplus|xiaomi|redmi|sm-/
	},
	laptop: { label: 'Laptop', icon: Laptop, pattern: /macbook|laptop|thinkpad|notebook|book|xps/ },
	tablet: { label: 'Tablet', icon: Tablet, pattern: /ipad|tab|kindle/ },
	computer: { label: 'Computer', icon: Monitor, pattern: /desktop|imac|pc|tower|workstation/ },
	tv: { label: 'TV', icon: Tv, pattern: /tv|chromecast|roku|fire-?stick|shield|appletv|bravia/ },
	console: {
		label: 'Console',
		icon: Gamepad2,
		pattern: /playstation|ps[45]|xbox|switch|nintendo|steam-?deck/
	},
	speaker: {
		label: 'Speaker',
		icon: Speaker,
		pattern: /sonos|echo|homepod|speaker|nest-?(audio|mini)/
	},
	watch: { label: 'Watch', icon: Watch, pattern: /watch/ },
	printer: {
		label: 'Printer',
		icon: Printer,
		pattern: /printer|epson|brother|canon|hp[a-z0-9-]*jet/
	},
	camera: { label: 'Camera', icon: Camera, pattern: /cam|doorbell|ring-/ },
	smart: {
		label: 'Smart home',
		icon: Lightbulb,
		pattern: /hue|bulb|plug|shelly|tasmota|esp|tuya|lamp/
	},
	server: { label: 'Server', icon: Server, pattern: /server|nas|synology|pi|proxmox|unraid/ },
	other: { label: 'Other', icon: MonitorSmartphone, pattern: /$^/ }
} as const;

export type Kind = keyof typeof kinds;

export function kindOf(device: Device): Kind {
	if (device.icon && device.icon in kinds) return device.icon as Kind;
	const name = `${device.customName ?? ''} ${device.hostname ?? ''}`.toLowerCase();
	const match = (Object.keys(kinds) as Kind[]).find((kind) => kinds[kind].pattern.test(name));
	return match ?? 'other';
}
