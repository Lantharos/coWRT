<script lang="ts">
	import { api } from '#lib/api/client.js';
	import { live } from '#lib/api/live.svelte.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { History, SystemInfo } from '#lib/api/types.js';
	import { bytes, duration, joined } from '#lib/format.js';
	import AreaChart from '#lib/components/charts/AreaChart.svelte';
	import Segmented from '#lib/components/ui/Segmented.svelte';
	import Block from '#lib/components/ui/Block.svelte';
	import Facts from '#lib/components/ui/Facts.svelte';
	import Section from '#lib/components/ui/Section.svelte';
	import Logs from '#lib/components/system/Logs.svelte';
	import Maintenance from '#lib/components/system/Maintenance.svelte';
	import Password from '#lib/components/system/Password.svelte';
	import RouterSettings from '#lib/components/system/RouterSettings.svelte';
	import Updates from '#lib/components/system/Updates.svelte';

	const system = poll(() => api.get<SystemInfo>('/system'), 60000);
	const history = poll(() => api.get<History>('/system/history'), 60000);

	let range = $state<'live' | 'day'>('live');

	const snapshot = $derived(live.snapshot);
	const memoryPercent = $derived(
		snapshot ? (snapshot.memory.used / snapshot.memory.total) * 100 : 0
	);
	const cpu = $derived(
		range === 'live'
			? live.samples.map((s) => s.cpu)
			: (history.data?.points.map((p) => p.cpu) ?? [])
	);
	const memory = $derived(
		range === 'live'
			? live.samples.map((s) => (s.memory.used / s.memory.total) * 100)
			: (history.data?.points.map((p) => p.memory) ?? [])
	);
	const storage = $derived(system.data?.storage);
	const storagePercent = $derived(storage ? (storage.used / Math.max(1, storage.total)) * 100 : 0);
</script>

<div>
	<h1 class="font-display text-[42px] leading-none font-medium">System</h1>
	<p class="mt-2 text-muted">
		{#if snapshot}Running for {duration(snapshot.uptime)}{:else}&nbsp;{/if}
	</p>
</div>

<Section title="Health">
	{#snippet actions()}
		<div class="w-48">
			<Segmented
				bind:value={range}
				options={[
					{ value: 'live', label: 'Live' },
					{ value: 'day', label: '24 hours' }
				]}
			/>
		</div>
	{/snippet}
	<div class="grid gap-[3px] md:grid-cols-2">
		<Block title="Processor" aside={cpuValue}>
			<AreaChart values={cpu} max={100} />
		</Block>
		<Block title="Memory" aside={memoryValue}>
			<AreaChart values={memory} max={100} />
		</Block>
	</div>
	<Block title="Storage" aside={storageValue}>
		<div class="h-2 overflow-hidden rounded-full bg-overlay">
			<div
				class="h-full rounded-full {storagePercent > 85 ? 'bg-danger' : 'bg-accent'}"
				style:width="{storagePercent}%"
			></div>
		</div>
	</Block>
</Section>

<Section title="This router">
	<Block>
		<Facts
			facts={[
				{ label: 'Name', value: system.data?.hostname },
				{ label: 'Model', value: system.data?.model },
				{ label: 'Processor', value: system.data?.processor },
				{ label: 'coWRT', value: system.data?.cowrt },
				{
					label: 'OpenWrt',
					value: system.data ? `${system.data.openwrt} (${system.data.revision})` : null
				},
				{ label: 'Kernel', value: system.data?.kernel },
				{ label: 'Platform', value: system.data?.target }
			]}
		/>
	</Block>
	{#if system.data}
		<RouterSettings system={system.data} onsaved={() => system.refresh()} />
	{/if}
</Section>

<Section title="Upkeep">
	<Updates current={system.data?.cowrt ?? ''} />
	<Maintenance />
</Section>

<Section title="Sign in">
	<Password />
</Section>

<Section title="Log">
	<Logs />
</Section>

{#snippet cpuValue()}
	<span class="font-display text-2xl">{Math.round(snapshot?.cpu ?? 0)}%</span>
{/snippet}

{#snippet memoryValue()}
	<span class="font-display text-2xl">{Math.round(memoryPercent)}%</span>
{/snippet}

{#snippet storageValue()}
	<span class="text-muted">
		{storage ? `${joined(bytes(storage.used))} of ${joined(bytes(storage.total))}` : ''}
	</span>
{/snippet}
