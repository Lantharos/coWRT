<script lang="ts">
	import Results from './Results.svelte';

	let { open = $bindable() }: { open: boolean } = $props();

	function typing(target: EventTarget | null) {
		return (
			target instanceof HTMLElement &&
			(target.isContentEditable || ['INPUT', 'TEXTAREA'].includes(target.tagName))
		);
	}

	function keydown(event: KeyboardEvent) {
		if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
			event.preventDefault();
			open = !open;
		} else if (event.key === '/' && !open && !typing(event.target)) {
			event.preventDefault();
			open = true;
		} else if (event.key === 'Escape' && open) {
			open = false;
		}
	}
</script>

<svelte:window onkeydown={keydown} />

{#if open}
	<div class="fixed inset-0 z-50 flex justify-center px-3 pt-[12vh]">
		<button
			type="button"
			aria-label="Close search"
			class="absolute inset-0 cursor-default bg-black/55 backdrop-blur-[2px]"
			onclick={() => (open = false)}
		></button>
		<div
			role="dialog"
			aria-modal="true"
			aria-label="Search"
			class="relative h-fit w-full max-w-[560px] animate-rise overflow-hidden rounded-panel bg-surface shadow-2xl shadow-black/60"
		>
			<Results onclose={() => (open = false)} />
		</div>
	</div>
{/if}
