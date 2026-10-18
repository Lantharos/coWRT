<script lang="ts">
	import type { Color } from '#lib/api/types.js';
	import { palette } from '#lib/palette.js';

	let { value = $bindable() }: { value: Color } = $props();
</script>

<div class="flex flex-col gap-1.5">
	<span class="text-[13px] text-muted">Color</span>
	<div class="flex gap-2">
		{#each Object.entries(palette) as [name, hex] (name)}
			<button
				type="button"
				aria-label={name}
				aria-pressed={value === name}
				onclick={() => (value = name as Color)}
				class="grid size-9 cursor-pointer place-items-center rounded-full transition-transform hover:scale-110 {value ===
				name
					? 'ring-2 ring-fg ring-offset-2 ring-offset-surface'
					: ''}"
				style:background={hex}
			></button>
		{/each}
	</div>
</div>
