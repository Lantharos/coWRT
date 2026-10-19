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
	internet: { checked: boolean; online: boolean; latencyMs: number | null; online6: boolean };
	usageToday: number;
	uplink: Uplink | null;
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
	zonename: string | null;
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
	owner: string | null;
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
	signals: number[];
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
	recent: number[];
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
	expires: number | null;
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

export interface Roaming {
	fast: boolean;
	steering: boolean;
}

export interface WifiOverview {
	radios: Radio[];
	networks: WifiNetwork[];
	roaming: Roaming;
}

export interface ChannelLoad {
	channel: number;
	networks: number;
	load: number;
}

export interface WifiScan {
	neighbors: { ssid: string; channel: number; signal: number; width: number }[];
	channels: ChannelLoad[];
	best: number | null;
}

export interface Applied {
	confirm: { token: string; seconds: number } | null;
}

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
	auto: boolean;
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

export type DnsProvider =
	'off' | 'cloudflare' | 'quad9' | 'google' | 'mullvad' | 'adguard' | 'nextdns';

export interface Protection {
	ads: string[];
	threats: string[];
	allow: string[];
	updated: number | null;
	privateDns: { provider: DnsProvider; nextdns: string | null };
}

export interface UpdateInfo {
	current: string;
	latest: string;
	notes: string | null;
	available: boolean;
}

export interface LogEntry {
	time: number;
	level: 'error' | 'warning' | 'info' | 'debug';
	message: string;
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

export interface Person {
	id: string;
	name: string;
	color: Color;
	blocked: boolean;
	schedules: Schedule[];
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

export interface SetupStatus {
	needed: boolean;
	passwordSet: boolean;
}

export interface SiteActivity {
	domain: string;
	lookups: number;
	blocked: number;
}

export interface ActivityCounts {
	lookups: number;
	blocked: number;
}

export interface Activity {
	days: { day: number; down: number; up: number }[];
	sites: SiteActivity[];
	today: ActivityCounts;
}

export interface ProtectionActivity {
	enabled: boolean;
	buckets: { time: number; lookups: number; blocked: number }[];
	topBlocked: SiteActivity[];
}

export type UplinkKind = 'cable' | 'phone' | 'other';

export interface Uplink {
	id: string;
	name: string;
	kind: UplinkKind;
	device: string | null;
	connected: boolean;
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
