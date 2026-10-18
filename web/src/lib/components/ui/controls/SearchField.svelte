<script lang="ts">
	import { Search } from '@lucide/svelte';

	type Props = { value: string; placeholder: string; tone?: 'surface' | 'raised'; class?: string };
	let {
		value = $bindable(),
		placeholder,
		tone = 'raised',
		class: className = ''
	}: Props = $props();

	const tones = {
		surface: 'bg-surface has-[:focus]:bg-raised',
		raised: 'bg-raised has-[:focus]:bg-overlay'
	};
</script>

<label
	class="flex h-11 items-center gap-2.5 rounded-control px-3.5 text-muted field-focus transition-[background-color] {tones[
		tone
	]} {className}"
>
	<Search size={17} class="shrink-0" />
	<input
		bind:value
		{placeholder}
		class="w-full bg-transparent text-[15px] text-fg outline-none placeholder:text-faint"
	/>
</label>
