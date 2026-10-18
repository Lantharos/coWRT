const RECENT = 6;

const levels: [number, string][] = [
	[-60, 'Great'],
	[-70, 'Good'],
	[-78, 'Fair']
];

export function recentSignal(signals: number[]) {
	const recent = signals.slice(-RECENT);
	if (!recent.length) return null;
	return Math.round(recent.reduce((sum, value) => sum + value, 0) / recent.length);
}

export function signalLabel(dbm: number) {
	return levels.find(([floor]) => dbm >= floor)?.[1] ?? 'Weak';
}

export const isWeak = (signals: number[]) => {
	const dbm = recentSignal(signals);
	return dbm !== null && signalLabel(dbm) === 'Weak';
};
