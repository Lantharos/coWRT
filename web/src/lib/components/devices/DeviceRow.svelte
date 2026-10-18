<script lang="ts">
	import type { Device } from '#lib/api/types.js';
	import { bandLabel, bits, joined } from '#lib/format.js';
	import Row from '#lib/components/ui/Row.svelte';
	import { kinds, kindOf } from './kinds';

	type Props = { device: Device; tint?: string; onselect: (device: Device) => void };
	let { device, tint, onselect }: Props = $props();

	const detail = $derived.by(() => {
		const connection = device.connection;
		const where =
			connection.kind === 'wifi'
				? `${bandLabel[connection.band]} Wi-Fi`
				: connection.kind === 'wired'
					? 'Cable'
					: 'Offline';
		return [device.ipv4, where].filter(Boolean).join(' · ');
	});
	const value = $derived.by(() => {
		if (device.controls.blocked) return { text: 'Paused', tone: 'bad' as const };
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
