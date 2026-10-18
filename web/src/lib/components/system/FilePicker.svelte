<script lang="ts">
	import type { Snippet } from 'svelte';
	import Button from '#lib/components/ui/controls/Button.svelte';

	type Props = { accept: string; onpick: (file: File) => void; children: Snippet };
	let { accept, onpick, children }: Props = $props();

	let input = $state<HTMLInputElement>();

	function picked(event: Event) {
		const target = event.currentTarget as HTMLInputElement;
		const file = target.files?.[0];
		if (file) onpick(file);
		target.value = '';
	}
</script>

<input bind:this={input} type="file" class="hidden" {accept} onchange={picked} />
<Button onclick={() => input?.click()} class="w-full">{@render children()}</Button>
