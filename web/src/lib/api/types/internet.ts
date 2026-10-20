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

export type QueueMode = 'fixed' | 'speedtest' | 'adaptive';

export interface SmartQueue {
	enabled: boolean;
	mode: QueueMode;
	download: number;
	upload: number;
}

export interface InternetOverview {
	connection: InternetConnection;
	ports: string[];
	qos: SmartQueue;
	acceleration: Acceleration;
	ipv6: Ipv6Settings;
}

export interface Ipv6Settings {
	enabled: boolean;
	prefix: number | null;
}

export interface SpeedResult {
	time: number;
	download: number;
	upload: number;
	latency: number;
	loadedLatency: number | null;
}

export interface SpeedProgress {
	phase: 'latency' | 'download' | 'upload';
	latency: number | null;
	download: number | null;
	upload: number | null;
	current: number;
	trace: number[];
}

export interface SpeedOverview {
	running: SpeedProgress | null;
	error: string | null;
	history: SpeedResult[];
}

export type UplinkKind = 'cable' | 'phone' | 'other';

export interface Uplink {
	id: string;
	name: string;
	kind: UplinkKind;
	device: string | null;
	connected: boolean;
	ipv6: boolean;
	online: boolean;
	latencyMs: number | null;
	inUse: boolean;
	since: number;
}

export interface UplinkStatus {
	mode: string;
	failback: boolean;
	active: string | null;
	uplinks: Uplink[];
	phoneDrivers: boolean;
}

export interface LinePoint {
	time: number;
	latency: number;
	p95: number;
	loss: number;
	down: number;
	up: number;
	limitDown: number | null;
	limitUp: number | null;
}

export interface LineView {
	now: {
		latencyMs: number | null;
		baselineMs: number | null;
		bloated: boolean;
		limits: { download: number; upload: number } | null;
	};
	bucketSecs: number;
	points: LinePoint[];
}
