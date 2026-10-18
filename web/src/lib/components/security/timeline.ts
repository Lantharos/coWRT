import type { ProtectionActivity } from '#lib/api/types.js';

const BUCKET_SECS = 600;

type Bucket = ProtectionActivity['buckets'][number];

export function timeline(
	buckets: Bucket[],
	slots: number,
	perSlot: number,
	pick: (b: Bucket) => number
) {
	const span = BUCKET_SECS * perSlot;
	const end = Math.floor(Date.now() / 1000 / span) * span;
	const values = new Array<number>(slots).fill(0);
	for (const bucket of buckets) {
		const index = slots - 1 - Math.floor((end - Math.floor(bucket.time / span) * span) / span);
		if (index >= 0 && index < slots) values[index] += pick(bucket);
	}
	return values;
}
