<script lang="ts">
	import type { Snippet } from 'svelte';

	type Props = { value: string; unit?: string; label: string; lead?: Snippet };
	let { value, unit, label, lead }: Props = $props();
</script>

<div class="flex flex-col gap-1">
	<div class="flex items-baseline gap-1.5">
		{#if lead}<span class="self-center">{@render lead()}</span>{/if}
		<span class="font-display text-[clamp(2.25rem,5vw,3.5rem)] leading-none font-normal"
			>{value}</span
		>
		{#if unit}<span class="font-display text-xl text-muted">{unit}</span>{/if}
	</div>
	<span class="text-[15px] text-muted">{label}</span>
</div>
