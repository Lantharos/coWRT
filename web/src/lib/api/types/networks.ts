export type Color = 'green' | 'blue' | 'purple' | 'orange' | 'pink' | 'teal';

export interface Dhcp {
	enabled: boolean;
	start: number;
	limit: number;
	leaseTime: string;
}

export interface Network {
	id: string;
	name: string;
	color: Color;
	primary: boolean;
	vlan: number;
	address: string | null;
	dhcp: Dhcp | null;
	trusted: boolean;
	internet: boolean;
	ports: string[];
	tagged: string[];
	wifi: string[];
	ipv6: { enabled: boolean; prefixes: string[] };
}

export interface NetworkInput {
	name: string;
	color: Color;
	vlan: number;
	address: string;
	dhcp: Dhcp;
	trusted: boolean;
	internet: boolean;
	ipv6: boolean;
}

export interface Port {
	name: string;
	label: string;
	up: boolean;
	speed: number | null;
	network: string | null;
	tagged: string[];
}

export interface NetworksOverview {
	networks: Network[];
	ports: Port[];
}

export interface LocalName {
	id: string;
	name: string;
	ip: string;
}

export interface StaticRoute {
	id: string;
	target: string;
	gateway: string;
	network: string;
}

export interface LanOverview {
	domain: string | null;
	names: LocalName[];
	routes: StaticRoute[];
}
