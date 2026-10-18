<script lang="ts" module>
	export type Metric = 'cpu' | 'memory';
</script>

<script lang="ts">
	import { api } from '#lib/api/client.js';
	import { live } from '#lib/api/live.svelte.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { History, Snapshot } from '#lib/api/types.js';
	import AreaChart from '#lib/components/charts/AreaChart.svelte';
	import Segmented from '#lib/components/ui/controls/Segmented.svelte';

	let { metric }: { metric: Metric } = $props();

	const history = poll(() => api.get<History>('/system/history'), 60000);

	let range = $state<'live' | 'day'>('live');

	const percent = (snapshot: Snapshot) =>
		metric === 'cpu' ? snapshot.cpu : (snapshot.memory.used / snapshot.memory.total) * 100;
	const values = $derived(
		range === 'live'
			? live.samples.map(percent)
			: (history.data?.points.map((point) => point[metric]) ?? [])
	);
	const average = $derived(values.length ? values.reduce((a, b) => a + b, 0) / values.length : 0);
	const peak = $derived(Math.max(0, ...values));
</script>

<div class="flex flex-col gap-6">
	<Segmented
		bind:value={range}
		options={[
			{ value: 'live', label: 'Last two minutes' },
			{ value: 'day', label: '24 hours' }
		]}
	/>
	<AreaChart {values} max={100} height={160} />
	<dl class="flex gap-10 text-[14px]">
		<div>
			<dt class="text-muted">Average</dt>
			<dd class="text-[17px]">{Math.round(average)}%</dd>
		</div>
		<div>
			<dt class="text-muted">Peak</dt>
			<dd class="text-[17px]">{Math.round(peak)}%</dd>
		</div>
	</dl>
</div>
