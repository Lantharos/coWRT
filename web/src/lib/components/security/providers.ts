import type { DnsProvider } from '#lib/api/types.js';

export const providers: { value: DnsProvider; label: string; detail?: string }[] = [
	{ value: 'off', label: 'Off', detail: 'Whatever your internet provider gives you' },
	{ value: 'cloudflare', label: 'Cloudflare', detail: 'Fast, logs wiped within a day' },
	{ value: 'quad9', label: 'Quad9', detail: 'Swiss, also blocks known malware' },
	{ value: 'mullvad', label: 'Mullvad', detail: 'No logs at all' },
	{ value: 'google', label: 'Google' },
	{ value: 'adguard', label: 'AdGuard', detail: 'Blocks ads at the source' },
	{ value: 'nextdns', label: 'NextDNS', detail: 'Your own filtering profile' }
];
