<script lang="ts">
	import { EthernetPort } from '@lucide/svelte';
	import type { Network, Port } from '#lib/api/types.js';
	import { networkColors } from './colors';

	type Props = { port: Port; networks: Network[]; onselect: (port: Port) => void };
	let { port, networks, onselect }: Props = $props();

	const network = $derived(networks.find((n) => n.id === port.network));
	const speed = $derived(
		port.speed ? (port.speed >= 1000 ? `${port.speed / 1000} Gbps` : `${port.speed} Mbps`) : null
	);
</script>

<button
	type="button"
	onclick={() => onselect(port)}
	class="flex cursor-pointer flex-col gap-4 bg-surface px-5 py-4 text-left transition-colors hover:bg-raised"
>
	<span class="flex items-center justify-between gap-3">
		<span class="font-display text-xl">{port.label}</span>
		<EthernetPort size={22} class={port.up ? 'text-accent' : 'text-faint'} />
	</span>
	<span class="flex flex-col gap-0.5">
		<span class="flex items-center gap-2 text-[15px]">
			{#if network}
				<span class="size-2.5 rounded-full" style:background={networkColors[network.color]}></span>
				{network.name}
			{:else}
				<span class="text-muted">Switched off</span>
			{/if}
			{#if port.tagged.length}
				<span class="text-[13px] text-muted">+{port.tagged.length}</span>
			{/if}
		</span>
		<span class="text-[13px] text-muted"
			>{port.up ? (speed ?? 'Connected') : 'Nothing plugged in'}</span
		>
	</span>
</button>
