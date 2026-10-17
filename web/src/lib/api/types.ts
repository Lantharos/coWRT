export type Band = '2g' | '5g' | '6g' | '60g';

export interface Wan {
	up: boolean;
	device: string | null;
	proto: string | null;
	uptime: number;
	ipv4: string | null;
	gateway: string | null;
	ipv6: string | null;
	prefix6: string | null;
	dns: string[];
}

export interface Snapshot {
	time: number;
	uptime: number;
	cpu: number;
	memory: { used: number; total: number };
	wan: Wan;
	throughput: { rx: number; tx: number };
	internet: { checked: boolean; online: boolean; latencyMs: number | null };
	usageToday: number;
}

export interface HistoryPoint {
	time: number;
	cpu: number;
	memory: number;
	rx: number;
	tx: number;
}

export interface History {
	bucketSecs: number;
	points: HistoryPoint[];
}

export interface SystemInfo {
	hostname: string;
	model: string;
	processor: string;
	kernel: string;
	openwrt: string;
	revision: string;
	target: string;
	cowrt: string;
	storage: { used: number; total: number };
	localTime: number;
}

export type Connection =
	| {
			kind: 'wifi';
			ssid: string;
			band: Band;
			iface: string;
			signal: number | null;
			rxRate: number;
			txRate: number;
	  }
	| { kind: 'wired' }
	| { kind: 'offline' };

export interface Device {
	mac: string;
	name: string;
	customName: string | null;
	hostname: string | null;
	icon: string | null;
	ipv4: string | null;
	online: boolean;
	connection: Connection;
	reservedIp: string | null;
	leaseExpires: number | null;
	isYou: boolean;
	network: string | null;
	ipv6: string[];
	controls: DeviceControls;
	usage: DeviceUsage;
}

export type Day = 'mon' | 'tue' | 'wed' | 'thu' | 'fri' | 'sat' | 'sun';

export interface Schedule {
	days: Day[];
	from: string;
	to: string;
}

export type Priority = 'low' | 'normal' | 'high';

export interface DeviceControls {
	blocked: boolean;
	schedules: Schedule[];
	limitDown: number | null;
	limitUp: number | null;
	priority: Priority;
}

export interface DeviceUsage {
	down: number;
	up: number;
	todayDown: number;
	todayUp: number;
}

export type Security = 'wpa3' | 'wpa2-wpa3' | 'wpa2' | 'enhanced-open' | 'open';

export interface WifiNetwork {
	id: string;
	ssid: string;
	security: Security;
	password: string | null;
	hidden: boolean;
	isolate: boolean;
	network: string;
	enabled: boolean;
	bands: Band[];
	clients: number;
}

export type WifiNetworkInput = Omit<WifiNetwork, 'id' | 'clients'>;

export interface Radio {
	name: string;
	band: Band;
	channel: string;
	htmode: string | null;
	txpower: number | null;
	country: string | null;
	enabled: boolean;
	up: boolean;
	live: {
		channel: number;
		frequency: number;
		txpower: number | null;
		noise: number | null;
		utilization: number | null;
	} | null;
	channels: number[];
	htmodes: string[];
}

export interface RadioInput {
	channel: string;
	htmode: string | null;
	txpower: number | null;
	country: string | null;
	enabled: boolean;
}

export interface WifiOverview {
	radios: Radio[];
	networks: WifiNetwork[];
}

export interface Applied {
	confirm: { token: string; seconds: number } | null;
}

export type NetworkColor = 'green' | 'blue' | 'purple' | 'orange' | 'pink' | 'teal';

export interface Dhcp {
	enabled: boolean;
	start: number;
	limit: number;
	leaseTime: string;
}

export interface Network {
	id: string;
	name: string;
	color: NetworkColor;
	primary: boolean;
	vlan: number;
	address: string | null;
	dhcp: Dhcp | null;
	trusted: boolean;
	internet: boolean;
	ports: string[];
	tagged: string[];
	wifi: string[];
}

export interface NetworkInput {
	name: string;
	color: NetworkColor;
	vlan: number;
	address: string;
	dhcp: Dhcp;
	trusted: boolean;
	internet: boolean;
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

export type ConnectionKind = 'dhcp' | 'static' | 'pppoe';
export type Acceleration = 'off' | 'software' | 'hardware';

export interface InternetConnection {
	kind: ConnectionKind;
	port: string;
	vlan: number | null;
	address: string | null;
	gateway: string | null;
	dns: string[];
	username: string | null;
	password: string | null;
	mac: string | null;
}

export interface SmartQueue {
	enabled: boolean;
	download: number;
	upload: number;
}

export interface InternetOverview {
	connection: InternetConnection;
	ports: string[];
	qos: SmartQueue;
	acceleration: Acceleration;
}

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
}

export type ForwardInput = Omit<Forward, 'id' | 'address'>;

export interface Reach {
	publicIp: string | null;
	wanIp: string | null;
	behindNat: boolean;
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
