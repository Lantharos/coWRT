<script lang="ts" generics="T extends { id: string }">
	import type { Snippet } from 'svelte';
	import { X } from '@lucide/svelte';

	type Props = {
		items: T[];
		empty: string;
		label: (item: T) => string;
		busy: boolean;
		onremove: (item: T) => void;
		children: Snippet<[T]>;
	};
	let { items, empty, label, busy, onremove, children }: Props = $props();
</script>

<div class="flex flex-col">
	{#each items as item (item.id)}
		<div class="flex min-h-12 items-center justify-between gap-3">
			<div class="min-w-0 text-[15px]">{@render children(item)}</div>
			<button
				type="button"
				aria-label="Remove {label(item)}"
				disabled={busy}
				onclick={() => onremove(item)}
				class="grid size-8 shrink-0 cursor-pointer place-items-center rounded-full text-faint hover:bg-raised hover:text-danger"
			>
				<X size={16} />
			</button>
		</div>
	{:else}
		<p class="py-2 text-[14px] text-muted">{empty}</p>
	{/each}
</div>
