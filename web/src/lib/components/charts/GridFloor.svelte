<script lang="ts">
	import { onMount } from 'svelte';

	let { levels }: { levels: number[] } = $props();

	const COLUMNS = 34;
	const ROWS = 22;
	const SAMPLE_MS = 1000;
	const GREEN = '52, 168, 83';
	const QUIET_BPS = 2_000_000;

	let canvas: HTMLCanvasElement;
	let field: number[] = [];
	let lastSample = 0;
	let kick = () => {};

	$effect(() => {
		const peak = Math.max(QUIET_BPS, ...levels);
		field = levels.map((level) => level / peak);
		lastSample = performance.now();
		kick();
	});

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

		function draw(now: number) {
			frame = 0;
			const drift = Math.min(1, (now - lastSample) / SAMPLE_MS);
			const horizon = height * 0.08;
			const level = (age: number) => {
				const newer = field.length - 1 - Math.floor(age);
				const blend = age - Math.floor(age);
				const at = (index: number) => (index >= 0 && index < field.length ? field[index] : 0);
				return at(newer) * (1 - blend) + at(newer - 1) * blend;
			};
			const point = (column: number, row: number) => {
				const age = row + drift;
				const x = column / COLUMNS - 0.5;
				const lift = level(Math.max(0, age)) * ridge(x, age) * Math.exp(-((x * 2.4) ** 2));
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

			if (drift < 1) kick();
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
