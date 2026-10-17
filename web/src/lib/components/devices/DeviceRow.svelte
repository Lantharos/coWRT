<script lang="ts">
	import { Cable } from '@lucide/svelte';
	import type { Device, Network } from '#lib/api/types.js';
	import { bandLabel } from '#lib/format.js';
	import { networkColors } from '#lib/components/networks/colors.js';
	import { kinds, kindOf } from './kinds';
	import SignalBars from './SignalBars.svelte';

	type Props = { device: Device; network?: Network; onselect?: (device: Device) => void };
	let { device, network, onselect }: Props = $props();

	const Icon = $derived(kinds[kindOf(device)].icon);
	const connection = $derived(device.connection);
</script>

<button
	type="button"
	onclick={() => onselect?.(device)}
	class="group flex w-full cursor-pointer items-center gap-4 bg-surface px-6 py-3.5 text-left transition-colors hover:bg-raised"
>
	<span
		class="grid size-10 shrink-0 place-items-center rounded-[12px] {device.online
			? 'bg-accent-soft text-accent'
			: 'bg-raised text-faint'}"
	>
		<Icon size={19} />
	</span>
	<span class="flex min-w-0 flex-1 flex-col">
		<span class="flex items-center gap-2 truncate text-[15px] font-medium">
			{device.name}
			{#if device.isYou}<span class="text-[13px] font-normal text-accent">This device</span>{/if}
		</span>
		<span class="flex items-center gap-1.5 truncate text-[13px] text-muted">
			{#if network}
				<span class="size-2 shrink-0 rounded-full" style:background={networkColors[network.color]}
				></span>
				{network.name} ·
			{/if}
			{device.ipv4 ?? 'No address'}{#if device.reservedIp}&nbsp;· reserved{/if}
		</span>
	</span>
	<span class="hidden shrink-0 items-center gap-2 text-[13px] text-muted sm:flex">
		{#if connection.kind === 'wifi'}
			<span class="truncate">{connection.ssid} · {bandLabel[connection.band]}</span>
			<span class="text-accent"><SignalBars dbm={connection.signal} /></span>
		{:else if connection.kind === 'wired'}
			<Cable size={15} /> Wired
		{:else}
			<span class="text-faint">Offline</span>
		{/if}
	</span>
</button>
