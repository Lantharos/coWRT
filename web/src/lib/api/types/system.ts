import type { Uplink } from './internet';

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
	updates: AvailableUpdates;
}

export interface AvailableUpdates {
	cowrt: string | null;
	openwrt: string | null;
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

export interface Applied {
	confirm: { token: string; seconds: number } | null;
}

export interface UpdateInfo {
	current: string;
	latest: string;
	notes: string | null;
	available: boolean;
	automatic: boolean;
}

export type UpgradeProgress =
	| { stage: 'idle' | 'preparing' | 'building' | 'verifying' | 'installing' }
	| { stage: 'failed'; reason: string };

export interface OpenWrtInfo {
	current: string;
	latest: string | null;
	available: boolean;
	progress: UpgradeProgress;
}

export interface LogEntry {
	time: number;
	level: 'error' | 'warning' | 'info' | 'debug';
	message: string;
}

export interface SetupStatus {
	needed: boolean;
	passwordSet: boolean;
}
