<script lang="ts">
	import type { Snippet } from 'svelte';

	type Props = {
		title?: string;
		description?: string;
		aside?: Snippet;
		children?: Snippet;
		class?: string;
	};
	let { title, description, aside, children, class: className = '' }: Props = $props();
</script>

<div class="bg-surface px-6 py-5 {className}">
	{#if title}
		<div class="flex items-center justify-between gap-6" class:mb-4={children}>
			<div class="min-w-0">
				<h3 class="text-[17px] font-medium">{title}</h3>
				{#if description}<p class="mt-0.5 text-sm text-muted">{description}</p>{/if}
			</div>
			{#if aside}{@render aside()}{/if}
		</div>
	{/if}
	{#if children}{@render children()}{/if}
</div>
