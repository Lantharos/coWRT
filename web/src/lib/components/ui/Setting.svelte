<script lang="ts">
	import type { Snippet } from 'svelte';

	type Props = { title: string; hint?: string; stacked?: boolean; children: Snippet };
	let { title, hint, stacked = false, children }: Props = $props();
</script>

<div class="flex gap-x-6 gap-y-3 py-3 {stacked ? 'flex-col' : 'items-center justify-between'}">
	<div class="min-w-0">
		<p class="text-[15px]">{title}</p>
		{#if hint}<p class="text-[13px] text-muted">{hint}</p>{/if}
	</div>
	<div class={stacked ? '' : 'shrink-0'}>{@render children()}</div>
</div>
