// Builds the maker table coWRT uses to name devices: bun dev/vendors.ts
const brands: [RegExp, string][] = [
	[/^Apple, Inc/i, 'Apple'],
	[/^Samsung Electronics/i, 'Samsung'],
	[/^Google, Inc|^Google LLC/i, 'Google'],
	[/^Nest Labs/i, 'Google Nest'],
	[/^Amazon Technologies/i, 'Amazon'],
	[/^Sonos/i, 'Sonos'],
	[/^Bose Corporation/i, 'Bose'],
	[/Xiaomi Communications|Beijing Xiaomi/i, 'Xiaomi'],
	[/^Huawei Technologies|^HUAWEI/i, 'Huawei'],
	[/^Honor Device/i, 'Honor'],
	[/^OnePlus/i, 'OnePlus'],
	[/OPPO Mobile/i, 'OPPO'],
	[/^vivo Mobile/i, 'vivo'],
	[/^Motorola Mobility/i, 'Motorola'],
	[/^LG Electronics/i, 'LG'],
	[/^Sony (Interactive|Corporation|Mobile)/i, 'Sony'],
	[/^Nintendo/i, 'Nintendo'],
	[/^Valve Corporation/i, 'Valve'],
	[/^Microsoft/i, 'Microsoft'],
	[/^Intel Corporate/i, 'Intel'],
	[/^Espressif/i, 'Espressif'],
	[/Tuya/i, 'Tuya'],
	[/^Allterco Robotics|^Shelly/i, 'Shelly'],
	[/^Signify|^Philips Lighting/i, 'Philips Hue'],
	[/^Raspberry Pi/i, 'Raspberry Pi'],
	[/^TP-LINK|^TP-Link/i, 'TP-Link'],
	[/^Ubiquiti/i, 'Ubiquiti'],
	[/^Roku/i, 'Roku'],
	[/^Ring LLC/i, 'Ring'],
	[/^ecobee/i, 'ecobee'],
	[/^iRobot/i, 'iRobot'],
	[/^Hewlett Packard|^HP Inc/i, 'HP'],
	[/^Brother Industries/i, 'Brother'],
	[/^Canon Inc/i, 'Canon'],
	[/^Seiko Epson/i, 'Epson'],
	[/^Dell Inc/i, 'Dell'],
	[/^LCFC|^Lenovo/i, 'Lenovo'],
	[/^ASUSTek/i, 'ASUS'],
	[/^NETGEAR/i, 'Netgear'],
	[/^Synology/i, 'Synology'],
	[/^QNAP/i, 'QNAP'],
	[/^Garmin/i, 'Garmin'],
	[/^Fitbit/i, 'Fitbit'],
	[/^Withings/i, 'Withings'],
	[/^NVIDIA/i, 'NVIDIA'],
	[/^Hisense/i, 'Hisense'],
	[/^TCL /i, 'TCL'],
	[/^Vizio/i, 'Vizio'],
	[/^Panasonic/i, 'Panasonic'],
	[/^Dyson/i, 'Dyson'],
	[/^Nothing Technology/i, 'Nothing'],
	[/^Fairphone/i, 'Fairphone']
];

const csv = await (await fetch('https://standards-oui.ieee.org/oui/oui.csv')).text();
const prefixes = new Map<string, string[]>();
for (const line of csv.split('\n').slice(1)) {
	const match = line.match(/^MA-L,([0-9A-F]{6}),("([^"]*)"|[^,]*)/);
	if (!match) continue;
	const organization = match[3] ?? match[2];
	const brand = brands.find(([pattern]) => pattern.test(organization))?.[1];
	if (brand) prefixes.set(brand, [...(prefixes.get(brand) ?? []), match[1]]);
}
const lines = [...prefixes].map(([brand, list]) => `${brand}\t${list.sort().join(' ')}`).sort();
const out = new URL('../daemon/src/devices/discovery/vendors.txt', import.meta.url).pathname;
await Bun.write(out, lines.join('\n') + '\n');
console.log(`${[...prefixes.values()].flat().length} prefixes for ${lines.length} makers written to ${out}`);
