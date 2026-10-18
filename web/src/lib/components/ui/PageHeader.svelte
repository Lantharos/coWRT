<script lang="ts">
	import type { Snippet } from 'svelte';

	type Props = { headline: Snippet; detail?: string; action?: Snippet };
	let { headline, detail, action }: Props = $props();
</script>

<header
	class="flex max-w-[860px] flex-col gap-6 pt-4 sm:flex-row sm:items-end sm:justify-between sm:px-3 sm:pt-10"
>
	<div class="flex min-w-0 flex-col gap-3">
		<h1
			class="font-display text-[clamp(1.9rem,4vw,2.75rem)] leading-[1.08] font-medium tracking-[-0.01em]"
		>
			{@render headline()}
		</h1>
		{#if detail}<p class="text-[17px] text-muted">{detail}</p>{/if}
	</div>
	{#if action}<div class="shrink-0">{@render action()}</div>{/if}
</header>
