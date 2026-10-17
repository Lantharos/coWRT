<script lang="ts">
	import type { Snippet } from 'svelte';
	import { X } from '@lucide/svelte';

	type Props = {
		open: boolean;
		title: string;
		onclose: () => void;
		header?: Snippet;
		footer?: Snippet;
		children: Snippet;
	};

	let { open, title, onclose, header, footer, children }: Props = $props();
</script>

<svelte:window onkeydown={(event) => open && event.key === 'Escape' && onclose()} />

{#if open}
	<div class="fixed inset-0 z-40 flex justify-end">
		<button
			type="button"
			aria-label="Close"
			class="absolute inset-0 cursor-default bg-black/55 backdrop-blur-[2px]"
			onclick={onclose}
		></button>
		<div
			role="dialog"
			aria-modal="true"
			aria-label={title}
			class="relative flex h-full w-full max-w-[480px] animate-slide-in flex-col bg-surface shadow-2xl shadow-black/60 sm:m-3 sm:h-[calc(100%-1.5rem)] sm:rounded-panel"
		>
			<div class="flex items-start justify-between gap-4 px-6 pt-6 pb-4">
				<div class="min-w-0 flex-1">
					{#if header}{@render header()}{:else}<h2 class="font-display text-2xl">{title}</h2>{/if}
				</div>
				<button
					type="button"
					onclick={onclose}
					aria-label="Close"
					class="grid size-9 shrink-0 cursor-pointer place-items-center rounded-full bg-raised text-muted hover:text-fg"
				>
					<X size={18} />
				</button>
			</div>
			<div class="flex-1 overflow-y-auto px-6 pb-6">
				{@render children()}
			</div>
			{#if footer}
				<div class="flex justify-end gap-2 px-6 py-4">{@render footer()}</div>
			{/if}
		</div>
	</div>
{/if}
