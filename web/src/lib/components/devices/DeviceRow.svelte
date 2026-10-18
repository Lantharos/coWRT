<script lang="ts">
	import type { Device, Person } from '#lib/api/types.js';
	import { bandLabel, bits, joined } from '#lib/format.js';
	import Row from '#lib/components/ui/Row.svelte';
	import { kinds, kindOf } from './kinds';
	import { isWeak } from './signal';

	type Props = {
		device: Device;
		owner?: Person;
		tint?: string;
		onselect: (device: Device) => void;
	};
	let { device, owner, tint, onselect }: Props = $props();

	const detail = $derived.by(() => {
		const connection = device.connection;
		const where =
			connection.kind === 'wifi'
				? `${bandLabel[connection.band]} Wi-Fi`
				: connection.kind === 'wired'
					? 'Cable'
					: 'Offline';
		const weak = connection.kind === 'wifi' && isWeak(device.signals) ? 'weak signal' : null;
		return [owner?.name, device.ipv4, where, weak].filter(Boolean).join(' · ');
	});
	const value = $derived.by(() => {
		if (device.controls.blocked || owner?.blocked) return { text: 'Paused', tone: 'bad' as const };
		if (!device.online) return null;
		return { text: joined(bits(device.usage.down + device.usage.up)), tone: 'muted' as const };
	});
</script>

<Row
	title={device.name}
	{detail}
	icon={kinds[kindOf(device)].icon}
	tint={device.online ? tint : undefined}
	graph={device.online ? device.usage.recent : undefined}
	value={value?.text}
	tone={value?.tone}
	dimmed={!device.online}
	onclick={() => onselect(device)}
/>
