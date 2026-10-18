import type { SpeedResult } from '#lib/api/types.js';

const levels: [number, string][] = [
	[30, 'Stays quick under load'],
	[100, 'Slows a little under load'],
	[300, 'Lags under load']
];

export function underLoad(result: SpeedResult) {
	if (result.loadedLatency === null) return null;
	const added = Math.max(0, result.loadedLatency - result.latency);
	return levels.find(([limit]) => added < limit)?.[1] ?? 'Lags badly under load';
}

export const mbps = (bits: number) =>
	bits >= 100_000_000 ? Math.round(bits / 1e6) : (bits / 1e6).toFixed(1);
