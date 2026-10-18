<script lang="ts">
	import type { Device } from '#lib/api/types.js';
	import { bandLabel, bits, joined } from '#lib/format.js';
	import Row from '#lib/components/ui/Row.svelte';
	import { kinds, kindOf } from './kinds';

	type Props = { device: Device; onselect: (device: Device) => void };
	let { device, onselect }: Props = $props();

	const Icon = $derived(kinds[kindOf(device)].icon);
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
		const rate = device.usage.down + device.usage.up;
		if (rate > 50_000) return { text: joined(bits(device.usage.down)), tone: 'muted' as const };
		return null;
	});
</script>

<Row
	title={device.name}
	{detail}
	value={value?.text}
	tone={value?.tone}
	dimmed={!device.online}
	onclick={() => onselect(device)}
>
	{#snippet leading()}
		<span
			class="grid size-10 place-items-center rounded-[12px] bg-surface {device.online
				? 'text-fg'
				: 'text-faint'}"
		>
			<Icon size={19} />
		</span>
	{/snippet}
</Row>
