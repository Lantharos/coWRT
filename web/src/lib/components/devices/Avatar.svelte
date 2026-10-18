<script lang="ts">
	import type { Person } from '#lib/api/types.js';
	import { palette } from '#lib/palette.js';

	let { person, size = 40 }: { person: Pick<Person, 'name' | 'color'>; size?: number } = $props();

	const color = $derived(palette[person.color]);
</script>

<span
	class="grid shrink-0 place-items-center rounded-full font-display font-medium"
	style:width="{size}px"
	style:height="{size}px"
	style:font-size="{size * 0.42}px"
	style:color
	style:background="color-mix(in srgb, {color} 18%, transparent)"
>
	{person.name.trim().charAt(0).toUpperCase() || '?'}
</span>
