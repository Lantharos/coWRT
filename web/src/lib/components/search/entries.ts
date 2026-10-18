import type { Component } from 'svelte';
import {
	Archive,
	ArrowUpCircle,
	Cpu,
	Gauge,
	Globe,
	KeyRound,
	Lock,
	MonitorSmartphone,
	Network,
	Power,
	ScrollText,
	Shield,
	Tags,
	User,
	Wifi
} from '@lucide/svelte';
import type { Device, NetworksOverview, Person, WifiOverview } from '#lib/api/types.js';
import { kinds, kindOf } from '#lib/components/devices/kinds.js';

export type Entry = {
	label: string;
	detail?: string;
	icon: Component<{ size?: number }>;
	href: string;
	group: string;
};

const pages: Entry[] = [
	{ label: 'Overview', icon: Gauge, href: '/', group: 'Pages' },
	{ label: 'Devices', icon: MonitorSmartphone, href: '/devices', group: 'Pages' },
	{ label: 'Wi-Fi', icon: Wifi, href: '/wifi', group: 'Pages' },
	{ label: 'Networks', icon: Network, href: '/networks', group: 'Pages' },
	{ label: 'Access', icon: Globe, href: '/access', group: 'Pages' },
	{ label: 'Security', icon: Shield, href: '/security', group: 'Pages' },
	{ label: 'System', icon: Cpu, href: '/system', group: 'Pages' }
];

const actions: Entry[] = [
	{ label: 'Run a speed test', icon: Gauge, href: '/networks?open=speed', group: 'Actions' },
	{ label: 'Local names', icon: Tags, href: '/networks?open=names', group: 'Actions' },
	{ label: 'Encrypted DNS', icon: Lock, href: '/security?open=dns', group: 'Actions' },
	{ label: 'VPN', icon: KeyRound, href: '/access?open=vpn', group: 'Actions' },
	{
		label: 'Check for updates',
		icon: ArrowUpCircle,
		href: '/system?open=updates',
		group: 'Actions'
	},
	{ label: 'Download a backup', icon: Archive, href: '/system?open=backups', group: 'Actions' },
	{ label: 'Change password', icon: KeyRound, href: '/system?open=password', group: 'Actions' },
	{ label: 'Read the log', icon: ScrollText, href: '/system?open=log', group: 'Actions' },
	{ label: 'Restart the router', icon: Power, href: '/system?open=restart', group: 'Actions' }
];

export type Sources = {
	devices: Device[];
	people: Person[];
	wifi: WifiOverview | null;
	networks: NetworksOverview | null;
};

export function entries({ devices, people, wifi, networks }: Sources): Entry[] {
	return [
		...pages,
		...people.map((person) => ({
			label: person.name,
			detail: 'Person',
			icon: User,
			href: `/devices?person=${person.id}`,
			group: 'People'
		})),
		...devices.map((device) => ({
			label: device.name,
			detail: [device.ipv4, device.online ? 'online' : null].filter(Boolean).join(' · '),
			icon: kinds[kindOf(device)].icon,
			href: `/devices?device=${device.mac}`,
			group: 'Devices'
		})),
		...(wifi?.networks ?? []).map((network) => ({
			label: network.ssid,
			detail: 'Wi-Fi',
			icon: Wifi,
			href: `/wifi?network=${network.id}`,
			group: 'Wi-Fi'
		})),
		...(networks?.networks ?? []).map((network) => ({
			label: network.name,
			detail: network.address?.split('/')[0],
			icon: Network,
			href: `/networks?network=${network.id}`,
			group: 'Networks'
		})),
		...actions
	];
}

export function matches(entry: Entry, query: string) {
	const needle = query.trim().toLowerCase();
	if (!needle) return entry.group === 'Pages' || entry.group === 'Actions';
	return [entry.label, entry.detail, entry.group].some((field) =>
		field?.toLowerCase().includes(needle)
	);
}
