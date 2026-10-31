<script lang="ts" module>
	export type Sample = { time: number; value: number };
</script>

<script lang="ts">
	import { onMount } from 'svelte';

	let { samples }: { samples: Sample[] } = $props();

	const COLUMNS = 34;
	const ROWS = 22;
	const GREEN = '52, 168, 83';
	const QUIET_BPS = 2_000_000;
	const LAG_SECONDS = 1.25;
	const CLOCK_JUMP_SECONDS = 5;
	const STALE_SECONDS = 3;
	const PEAK_EASE_SECONDS = 0.6;

	let canvas: HTMLCanvasElement;
	let offset: number | null = null;
	let kick = () => {};

	const seconds = () => performance.now() / 1000;

	$effect(() => {
		const newest = samples.at(-1);
		if (!newest) return;
		const arrival = seconds() - newest.time;
		if (offset === null || arrival < offset || arrival - offset > CLOCK_JUMP_SECONDS)
			offset = arrival;
		kick();
	});

	function valueAt(time: number) {
		if (!samples.length || time < samples[0].time) return 0;
		let index = samples.length - 1;
		while (index > 0 && samples[index].time > time) index--;
		const current = samples[index];
		const next = samples[index + 1];
		if (!next) return current.value;
		const blend = (time - current.time) / (next.time - current.time);
		return current.value + (next.value - current.value) * blend;
	}

	function ridge(x: number, z: number) {
		return (
			0.55 +
			0.25 * Math.sin(x * 2.1 + z * 0.35) +
			0.2 * Math.sin(x * 5.3 - z * 0.6) * Math.cos(x * 1.3)
		);
	}

	onMount(() => {
		const context = canvas.getContext('2d')!;
		let frame = 0;
		let visible = true;
		let width = 0;
		let height = 0;
		let columnStroke: CanvasGradient | string = '';

		function resize() {
			const ratio = devicePixelRatio;
			width = canvas.clientWidth;
			height = canvas.clientHeight;
			canvas.width = width * ratio;
			canvas.height = height * ratio;
			context.setTransform(ratio, 0, 0, ratio, 0, 0);
			const gradient = context.createLinearGradient(0, height * 0.08, 0, height);
			gradient.addColorStop(0, `rgba(${GREEN}, 0.04)`);
			gradient.addColorStop(1, `rgba(${GREEN}, 0.7)`);
			columnStroke = gradient;
		}

		let peak = QUIET_BPS;
		let drawnAt = 0;

		function draw(now: number) {
			frame = 0;
			const newest = samples.at(-1);
			if (!newest || offset === null) return;
			const renderTime = now / 1000 - offset - LAG_SECONDS;
			const drift = renderTime - Math.floor(renderTime);
			const target = Math.max(QUIET_BPS, ...samples.slice(-ROWS - 2).map((sample) => sample.value));
			const elapsed = drawnAt ? (now - drawnAt) / 1000 : 0;
			peak += (target - peak) * (1 - Math.exp(-elapsed / PEAK_EASE_SECONDS));
			drawnAt = now;

			const horizon = height * 0.08;
			const point = (column: number, row: number) => {
				const age = row + drift;
				const x = column / COLUMNS - 0.5;
				const level = valueAt(Math.floor(renderTime) - row) / peak;
				const lift = level * ridge(x, age) * Math.exp(-((x * 2.4) ** 2));
				const depth = 1 / (1 + age * 0.16);
				return [
					width / 2 + x * 1.6 * width * 0.9 * depth,
					horizon + (height - horizon) * depth - lift * depth * height * 0.55
				];
			};

			context.clearRect(0, 0, width, height);
			context.lineWidth = 1;
			for (let row = ROWS; row >= -1; row--) {
				context.strokeStyle = `rgba(${GREEN}, ${0.1 + (1 - row / ROWS) * 0.7})`;
				context.beginPath();
				for (let column = 0; column <= COLUMNS; column++) {
					const [x, y] = point(column, row);
					if (column === 0) context.moveTo(x, y);
					else context.lineTo(x, y);
				}
				context.stroke();
			}
			context.strokeStyle = columnStroke;
			context.beginPath();
			for (let column = 0; column <= COLUMNS; column++) {
				for (let row = -1; row <= ROWS; row++) {
					const [x, y] = point(column, row);
					if (row === -1) context.moveTo(x, y);
					else context.lineTo(x, y);
				}
			}
			context.stroke();

			if (renderTime < newest.time + STALE_SECONDS) kick();
			else drawnAt = 0;
		}

		kick = () => {
			if (visible && !frame) frame = requestAnimationFrame(draw);
		};

		const resizer = new ResizeObserver(() => {
			resize();
			kick();
		});
		const observer = new IntersectionObserver(([entry]) => {
			visible = entry.isIntersecting;
			kick();
		});
		resizer.observe(canvas);
		observer.observe(canvas);

		return () => {
			cancelAnimationFrame(frame);
			resizer.disconnect();
			observer.disconnect();
			kick = () => {};
		};
	});
</script>

<canvas bind:this={canvas} class="block h-full w-full" aria-hidden="true"></canvas>
