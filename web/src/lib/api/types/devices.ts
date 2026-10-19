import type { Color } from './networks';
import type { Band } from './wifi';

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

export interface Person {
	id: string;
	name: string;
	color: Color;
	blocked: boolean;
	schedules: Schedule[];
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
