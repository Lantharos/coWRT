import type { SiteActivity } from './devices';

export type DnsProvider =
	'off' | 'cloudflare' | 'quad9' | 'google' | 'mullvad' | 'adguard' | 'nextdns';

export interface Protection {
	ads: string[];
	threats: string[];
	allow: string[];
	updated: number | null;
	privateDns: { provider: DnsProvider; nextdns: string | null };
	dnsFallback: boolean;
}

export interface ProtectionActivity {
	enabled: boolean;
	buckets: { time: number; lookups: number; blocked: number }[];
	topBlocked: SiteActivity[];
}
