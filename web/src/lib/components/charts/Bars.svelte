<script lang="ts">
	type Props = { values: number[]; height?: number; highlightLast?: boolean };
	let { values, height = 64, highlightLast = true }: Props = $props();

	const top = $derived(Math.max(1, ...values));
</script>

<div class="flex items-end gap-[3px]" style:height="{height}px" aria-hidden="true">
	{#each values as value, index (index)}
		<div
			class="min-w-0 flex-1 rounded-t-[3px] {highlightLast && index === values.length - 1
				? 'bg-accent'
				: value
					? 'bg-accent/45'
					: 'bg-overlay'}"
			style:height="{Math.max(4, (value / top) * 100)}%"
		></div>
	{/each}
</div>
