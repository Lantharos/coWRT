<script lang="ts">
	type Props = { values: number[]; color?: string; floor?: number };
	let { values, color = 'var(--color-accent)', floor = 200_000 }: Props = $props();

	const WIDTH = 72;
	const HEIGHT = 24;

	const path = $derived.by(() => {
		if (values.length < 2) return null;
		const top = Math.max(floor, ...values);
		const step = WIDTH / (values.length - 1);
		const points = values.map(
			(value, index) =>
				`${(index * step).toFixed(1)},${(HEIGHT - 1 - (value / top) * (HEIGHT - 3)).toFixed(1)}`
		);
		return {
			line: `M${points.join('L')}`,
			area: `M0,${HEIGHT}L${points.join('L')}L${WIDTH},${HEIGHT}Z`
		};
	});
</script>

<svg
	width={WIDTH}
	height={HEIGHT}
	viewBox="0 0 {WIDTH} {HEIGHT}"
	class="block shrink-0"
	aria-hidden="true"
>
	{#if path}
		<path d={path.area} fill={color} opacity="0.14" />
		<path d={path.line} fill="none" stroke={color} stroke-width="1.5" stroke-linejoin="round" />
	{:else}
		<line x1="0" y1={HEIGHT - 1} x2={WIDTH} y2={HEIGHT - 1} stroke="var(--color-overlay)" />
	{/if}
</svg>
