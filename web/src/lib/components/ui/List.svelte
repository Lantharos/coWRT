<script lang="ts">
	import type { Snippet } from 'svelte';

	type Props = { label?: string; aside?: Snippet; children: Snippet };
	let { label, aside, children }: Props = $props();
</script>

<section class="flex flex-col gap-1">
	{#if label || aside}
		<div class="flex min-h-8 items-center justify-between gap-4 px-3 pb-1">
			{#if label}<h2 class="text-[15px] text-muted">{label}</h2>{/if}
			{#if aside}{@render aside()}{/if}
		</div>
	{/if}
	{@render children()}
</section>
