<script lang="ts">
	import type { Network, Port } from '#lib/api/types.js';
	import { networkColors } from './colors.js';

	type Props = {
		ports: Port[];
		networks: Network[];
		wan: string | null;
		onselect: (port: Port) => void;
	};
	let { ports, networks, wan, onselect }: Props = $props();

	const colorOf = (port: Port) => {
		const network = networks.find((n) => n.id === port.network);
		return network ? networkColors[network.color] : null;
	};
</script>

<div class="flex flex-wrap gap-2 px-3 sm:gap-3">
	{#if wan}
		<div class="flex w-[58px] sm:w-[72px] flex-col items-center gap-2">
			<span class="relative grid h-14 w-full place-items-center rounded-[14px] bg-surface">
				<span class="h-4 w-7 rounded-[4px] bg-bg"></span>
				<span class="absolute top-2 right-2 size-1.5 rounded-full bg-accent"></span>
			</span>
			<span class="text-[13px] text-muted">Internet</span>
		</div>
		<span class="mx-1 my-2 w-px bg-line"></span>
	{/if}
	{#each ports as port (port.name)}
		{@const color = colorOf(port)}
		<button
			type="button"
			onclick={() => onselect(port)}
			class="group flex w-[58px] sm:w-[72px] cursor-pointer flex-col items-center gap-2"
		>
			<span
				class="relative grid h-14 w-full place-items-center overflow-hidden rounded-[14px] bg-surface transition-colors group-hover:bg-raised"
			>
				<span class="h-4 w-7 rounded-[4px] bg-bg"></span>
				<span class="absolute inset-x-0 bottom-0 h-1.5" style:background={color ?? 'transparent'}
				></span>
				{#if port.up}<span class="absolute top-2 right-2 size-1.5 rounded-full bg-accent"
					></span>{/if}
			</span>
			<span class="text-[13px] {color ? 'text-fg' : 'text-faint'}">{port.label}</span>
		</button>
	{/each}
</div>
