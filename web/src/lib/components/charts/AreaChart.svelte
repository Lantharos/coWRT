<script lang="ts">
	type Props = { values: number[]; max?: number; height?: number };
	let { values, max, height = 120 }: Props = $props();

	const WIDTH = 600;

	const path = $derived.by(() => {
		if (values.length < 2) return { line: '', area: '' };
		const top = max ?? Math.max(1, ...values) * 1.15;
		const step = WIDTH / (values.length - 1);
		const points = values.map(
			(value, index) =>
				`${(index * step).toFixed(1)},${(height - (value / top) * height).toFixed(1)}`
		);
		const line = `M${points.join('L')}`;
		return { line, area: `${line}L${WIDTH},${height}L0,${height}Z` };
	});
</script>

<svg
	viewBox="0 0 {WIDTH} {height}"
	preserveAspectRatio="none"
	class="block w-full"
	style:height="{height}px"
	aria-hidden="true"
>
	<path d={path.area} class="fill-accent/12" />
	<path
		d={path.line}
		class="fill-none stroke-accent"
		stroke-width="1.5"
		vector-effect="non-scaling-stroke"
	/>
</svg>
