import { Cpu, Gauge, Globe, MonitorSmartphone, Network, Shield, Wifi } from '@lucide/svelte';

export const tabs = [
	{ href: '/', label: 'Overview', icon: Gauge },
	{ href: '/devices', label: 'Devices', icon: MonitorSmartphone },
	{ href: '/wifi', label: 'Wi-Fi', icon: Wifi },
	{ href: '/networks', label: 'Networks', icon: Network },
	{ href: '/access', label: 'Access', icon: Globe },
	{ href: '/security', label: 'Security', icon: Shield },
	{ href: '/system', label: 'System', icon: Cpu }
];

export const isTab = (href: string, pathname: string) =>
	href === '/' ? pathname === '/' : pathname.startsWith(href);

export const tabIndex = (pathname: string) => tabs.findIndex((tab) => isTab(tab.href, pathname));
