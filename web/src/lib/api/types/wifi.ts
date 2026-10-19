export type Band = '2g' | '5g' | '6g' | '60g';

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
