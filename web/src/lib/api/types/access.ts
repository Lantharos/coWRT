export type ForwardProtocol = 'tcp' | 'udp' | 'both';

export interface Forward {
	id: string;
	name: string;
	mac: string;
	protocol: ForwardProtocol;
	external: string;
	internal: string | null;
	enabled: boolean;
	address: string | null;
	ipv6: boolean;
}

export type ForwardInput = Omit<Forward, 'id' | 'address' | 'ipv6'>;

export interface Reach {
	publicIp: string | null;
	wanIp: string | null;
	behindNat: boolean;
	reachable: boolean | null;
}

export interface UpnpMapping {
	protocol: string;
	external: number;
	address: string;
	internal: number;
	description: string;
}

export interface Upnp {
	installed: boolean;
	enabled: boolean;
	mappings: UpnpMapping[];
}

export interface Ddns {
	enabled: boolean;
	hostname: string;
	hasToken: boolean;
	status: {
		ipv4: string | null;
		ipv6: string | null;
		updated: number | null;
		error: string | null;
	};
}

export interface SharingOverview {
	forwards: Forward[];
	reach: Reach;
	upnp: Upnp;
	ddns: Ddns;
}

export interface PeerStatus {
	endpoint: string | null;
	handshake: number | null;
	received: number;
	sent: number;
}

export interface VpnPeer {
	id: string;
	name: string;
	address: string;
	status: PeerStatus;
}

export interface VpnServer {
	enabled: boolean;
	port: number;
	access: 'everything' | 'internet';
	endpoint: string | null;
	peers: VpnPeer[];
}

export interface VpnProvider {
	configured: boolean;
	enabled: boolean;
	name: string;
	endpoint: string | null;
	networks: string[];
	status: PeerStatus;
}

export interface VpnOverview {
	server: VpnServer;
	client: VpnProvider;
}
