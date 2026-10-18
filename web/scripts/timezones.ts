// Regenerates src/lib/data/timezones.json from the system's tzdata: bun scripts/timezones.ts
import { readFileSync, writeFileSync } from 'node:fs';

function posixRule(zone: string) {
	const data = readFileSync(`/usr/share/zoneinfo/${zone}`);
	const footer = data
		.subarray(data.lastIndexOf(10, data.length - 2) + 1)
		.toString()
		.trim();
	return footer || null;
}

const zones = Intl.supportedValuesOf('timeZone')
	.map((zone) => ({ zone, rule: posixRule(zone) }))
	.filter((entry): entry is { zone: string; rule: string } => entry.rule !== null);

zones.unshift({ zone: 'UTC', rule: 'GMT0' });
writeFileSync(new URL('../src/lib/data/timezones.json', import.meta.url), JSON.stringify(zones));
console.log(`${zones.length} time zones`);
