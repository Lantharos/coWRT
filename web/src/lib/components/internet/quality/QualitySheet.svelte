<script lang="ts">
	import type { LineView } from '#lib/api/types.js';
	import { bits, joined } from '#lib/format.js';
	import AreaChart from '#lib/components/charts/AreaChart.svelte';
	import Segmented from '#lib/components/ui/controls/Segmented.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import { average, median, worstHours } from './summary';

	type Props = { line: LineView | null; open: boolean; onclose: () => void };
	let { line, open, onclose }: Props = $props();

	let range = $state<'day' | 'week'>('day');

	const points = $derived.by(() => {
		const since = Date.now() / 1000 - (range === 'day' ? 86400 : 7 * 86400);
		return (line?.points ?? []).filter((point) => point.time >= since);
	});
	const latencies = $derived(points.map((point) => point.latency));
	const losses = $derived(points.map((point) => point.loss));
	const downloads = $derived(points.map((point) => point.down));
	const worst = $derived(worstHours(line?.points ?? []));

	const ms = (value: number) => `${Math.round(value)} ms`;
	const hour = (value: number) => `${String(value).padStart(2, '0')}:00`;
	const mbps = (kbit: number) => Math.round(kbit / 1000);

	const headline = $derived.by(() => {
		const now = line?.now;
		if (!now?.latencyMs) return 'Measuring your line…';
		const quiet = now.baselineMs ? `, ${ms(now.baselineMs)} when it's quiet` : '';
		return now.bloated
			? `Delayed right now: ${ms(now.latencyMs)}${quiet}.`
			: `${ms(now.latencyMs)} right now${quiet}.`;
	});
</script>

<Sheet {open} title="Line quality" {onclose}>
	<div class="flex flex-col gap-8">
		<div class="flex flex-col gap-1.5">
			<p class="text-[15px]">{headline}</p>
			{#if line?.now.limits}
				<p class="text-[14px] text-muted">
					The smart queue is letting through {mbps(line.now.limits.download)} Mbps down and
					{mbps(line.now.limits.upload)} Mbps up right now.
				</p>
			{/if}
		</div>

		<Segmented
			bind:value={range}
			options={[
				{ value: 'day', label: '24 hours' },
				{ value: 'week', label: '7 days' }
			]}
		/>

		<section class="flex flex-col gap-3">
			<h3 class="text-[15px]">Delay</h3>
			<AreaChart values={latencies} height={120} />
			<dl class="flex gap-10 text-[14px]">
				<div>
					<dt class="text-muted">Typical</dt>
					<dd class="text-[17px]">{ms(median(latencies))}</dd>
				</div>
				<div>
					<dt class="text-muted">Worst</dt>
					<dd class="text-[17px]">{ms(Math.max(0, ...points.map((point) => point.p95)))}</dd>
				</div>
			</dl>
		</section>

		<section class="flex flex-col gap-3">
			<h3 class="text-[15px]">Lost pings</h3>
			<AreaChart values={losses} max={Math.max(5, ...losses) * 1.15} height={80} />
			<dl class="flex gap-10 text-[14px]">
				<div>
					<dt class="text-muted">Average</dt>
					<dd class="text-[17px]">{average(losses).toFixed(1)}%</dd>
				</div>
				<div>
					<dt class="text-muted">Worst</dt>
					<dd class="text-[17px]">{Math.max(0, ...losses).toFixed(1)}%</dd>
				</div>
			</dl>
		</section>

		<section class="flex flex-col gap-3">
			<h3 class="text-[15px]">Speed used</h3>
			<AreaChart values={downloads} height={80} />
			<dl class="flex gap-10 text-[14px]">
				<div>
					<dt class="text-muted">Busiest download</dt>
					<dd class="text-[17px]">{joined(bits(Math.max(0, ...downloads)))}</dd>
				</div>
				<div>
					<dt class="text-muted">Busiest upload</dt>
					<dd class="text-[17px]">
						{joined(bits(Math.max(0, ...points.map((point) => point.up))))}
					</dd>
				</div>
			</dl>
		</section>

		<section class="flex flex-col gap-1.5">
			<h3 class="text-[15px]">Time of day</h3>
			<p class="text-[14px] text-muted">
				{#if worst === null}
					After a day of measuring, this shows when your line struggles most.
				{:else if worst === 'steady'}
					Your line holds up the same all day.
				{:else}
					Worst between {hour(worst.from)} and {hour(worst.to)}: {worst.loss.toFixed(1)}% of pings
					lost and {ms(worst.extraMs)} of extra delay.
				{/if}
			</p>
		</section>
	</div>
</Sheet>
