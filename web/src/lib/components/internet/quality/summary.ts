import type { LinePoint } from '#lib/api/types.js';

const LOSS_WEIGHT_MS = 4;
const NOTICEABLE = 5;
const NEIGHBOUR_SHARE = 0.6;
const HOURS_NEEDED = 20;

export interface WorstHours {
	from: number;
	to: number;
	loss: number;
	extraMs: number;
}

export const average = (values: number[]) =>
	values.length ? values.reduce((a, b) => a + b, 0) / values.length : 0;

export function median(values: number[]) {
	if (!values.length) return 0;
	const sorted = [...values].sort((a, b) => a - b);
	return sorted[Math.floor(sorted.length / 2)];
}

export function recentLoss(points: LinePoint[], seconds: number) {
	const since = Date.now() / 1000 - seconds;
	return average(points.filter((point) => point.time >= since).map((point) => point.loss));
}

export function worstHours(points: LinePoint[]): WorstHours | 'steady' | null {
	const hours = Array.from({ length: 24 }, () => ({
		loss: [] as number[],
		latency: [] as number[]
	}));
	for (const point of points) {
		const hour = new Date(point.time * 1000).getHours();
		hours[hour].loss.push(point.loss);
		hours[hour].latency.push(point.latency);
	}
	const measured = hours.filter((hour) => hour.latency.length);
	if (measured.length < HOURS_NEEDED) return null;
	const baseline = Math.min(...measured.map((hour) => average(hour.latency)));
	const stats = hours.map((hour) => {
		const loss = average(hour.loss);
		const extraMs = hour.latency.length ? average(hour.latency) - baseline : 0;
		return { loss, extraMs, score: loss * LOSS_WEIGHT_MS + extraMs };
	});
	const worst = stats.reduce(
		(best, stat, hour) => (stat.score > stats[best].score ? hour : best),
		0
	);
	if (stats[worst].score < NOTICEABLE) return 'steady';
	const threshold = stats[worst].score * NEIGHBOUR_SHARE;
	const window = [worst];
	const before = (hour: number) => (hour + 23) % 24;
	const after = (hour: number) => (hour + 1) % 24;
	for (
		let hour = before(worst);
		window.length < 24 && stats[hour].score >= threshold;
		hour = before(hour)
	) {
		window.unshift(hour);
	}
	for (
		let hour = after(worst);
		window.length < 24 && stats[hour].score >= threshold;
		hour = after(hour)
	) {
		window.push(hour);
	}
	return {
		from: window[0],
		to: after(window[window.length - 1]),
		loss: average(window.map((hour) => stats[hour].loss)),
		extraMs: average(window.map((hour) => stats[hour].extraMs))
	};
}
