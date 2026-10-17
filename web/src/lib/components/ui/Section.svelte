<script lang="ts">
	import type { Snippet } from 'svelte';

	type Props = { title: string; actions?: Snippet; children: Snippet };
	let { title, actions, children }: Props = $props();
</script>

<section class="flex flex-col gap-4">
	<div class="flex items-end justify-between gap-4 px-1">
		<h2 class="font-display text-[26px] leading-none font-medium">{title}</h2>
		{#if actions}<div class="flex gap-2">{@render actions()}</div>{/if}
	</div>
	<div class="flex flex-col gap-[3px] overflow-hidden rounded-panel">
		{@render children()}
	</div>
</section>
