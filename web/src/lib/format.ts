const BYTE_UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];
const BIT_UNITS = ['bps', 'Kbps', 'Mbps', 'Gbps'];

function scale(value: number, units: string[], base: number) {
	let index = 0;
	while (value >= base && index < units.length - 1) {
		value /= base;
		index++;
	}
	const digits = value >= 100 || index === 0 ? 0 : 1;
	return { value: value.toFixed(digits), unit: units[index] };
}

export const bytes = (value: number) => scale(value, BYTE_UNITS, 1000);
export const bits = (value: number) => scale(value, BIT_UNITS, 1000);

export function joined({ value, unit }: { value: string; unit: string }) {
	return `${value} ${unit}`;
}

export function duration(seconds: number) {
	const days = Math.floor(seconds / 86400);
	const hours = Math.floor((seconds % 86400) / 3600);
	const minutes = Math.floor((seconds % 3600) / 60);
	if (days > 0) return `${days}d ${hours}h`;
	if (hours > 0) return `${hours}h ${minutes}m`;
	return `${minutes}m`;
}

export function linkRate(kbps: number) {
	return kbps >= 1_000_000
		? `${(kbps / 1_000_000).toFixed(1)} Gbps`
		: `${Math.round(kbps / 1000)} Mbps`;
}

export const bandLabel: Record<string, string> = {
	'2g': '2.4 GHz',
	'5g': '5 GHz',
	'6g': '6 GHz',
	'60g': '60 GHz'
};

export function ago(unixSeconds: number) {
	const seconds = Date.now() / 1000 - unixSeconds;
	if (seconds < 60) return 'just now';
	if (seconds < 3600) return `${Math.round(seconds / 60)} min ago`;
	if (seconds < 86400) return `${Math.round(seconds / 3600)} h ago`;
	return `${Math.round(seconds / 86400)} days ago`;
}
