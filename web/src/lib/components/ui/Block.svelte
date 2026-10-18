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
	{#if title || description}
		<div class="flex items-center justify-between gap-6" class:mb-4={children}>
			<div class="min-w-0">
				{#if title}<h3 class="text-[17px] font-medium">{title}</h3>{/if}
				{#if description}
					<p class="text-sm text-muted" class:mt-0.5={title}>{description}</p>
				{/if}
			</div>
			{#if aside}{@render aside()}{/if}
		</div>
	{/if}
	{#if children}{@render children()}{/if}
</div>
