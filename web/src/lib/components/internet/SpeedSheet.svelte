<script lang="ts">
	import { speedtests } from '#lib/api/speedtest.svelte.js';
	import { ago } from '#lib/format.js';
	import AreaChart from '#lib/components/charts/AreaChart.svelte';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import { mbps, underLoad } from './grade';

	let { open, onclose }: { open: boolean; onclose: () => void } = $props();

	const running = $derived(speedtests.data?.running ?? null);
	const latest = $derived(speedtests.latest);
	const history = $derived([...(speedtests.data?.history ?? [])].reverse());
	const phaseLabel = { latency: 'Checking latency', download: 'Download', upload: 'Upload' };
</script>

<Sheet {open} title="Speed test" {onclose}>
	<div class="flex flex-col gap-8">
		{#if running}
			<div class="flex flex-col gap-3">
				<p class="text-[14px] text-muted">{phaseLabel[running.phase]}</p>
				<p class="flex items-baseline gap-2 font-display">
					<span class="text-5xl">{mbps(running.current)}</span>
					<span class="text-lg text-muted">Mbps</span>
				</p>
				<AreaChart values={running.trace} height={72} />
			</div>
		{:else if latest}
			<div class="flex flex-col gap-1">
				<p class="text-[14px] text-muted">{ago(latest.time)}</p>
				<div class="grid grid-cols-2 gap-y-5 pt-2">
					{@render figure('Download', mbps(latest.download), 'Mbps')}
					{@render figure('Upload', mbps(latest.upload), 'Mbps')}
					{@render figure('Latency', Math.round(latest.latency), 'ms')}
					{@render figure(
						'Under load',
						latest.loadedLatency === null ? '–' : Math.round(latest.loadedLatency),
						'ms'
					)}
				</div>
				{#if underLoad(latest)}
					<p class="pt-4 text-[14px] text-muted">{underLoad(latest)}.</p>
				{/if}
			</div>
		{:else}
			<p class="text-[14px] text-muted">
				Measures your line from the router itself, so Wi-Fi doesn't get in the way.
			</p>
		{/if}

		{#if history.length > 1}
			<div class="flex flex-col">
				<h3 class="pb-1 text-[15px]">Earlier</h3>
				{#each history.slice(running ? 0 : 1) as result (result.time)}
					<div class="flex justify-between gap-4 py-1.5 text-[14px]">
						<span class="text-muted">{ago(result.time)}</span>
						<span>
							{mbps(result.download)} / {mbps(result.upload)} Mbps
							<span class="text-muted">· {Math.round(result.latency)} ms</span>
						</span>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	{#snippet footer()}
		<Button variant="primary" onclick={() => speedtests.start()} busy={!!running}>
			{latest ? 'Test again' : 'Start'}
		</Button>
	{/snippet}
</Sheet>

{#snippet figure(label: string, value: string | number, unit: string)}
	<div class="flex flex-col">
		<span class="text-[13px] text-muted">{label}</span>
		<span class="flex items-baseline gap-1.5 font-display">
			<span class="text-3xl">{value}</span>
			<span class="text-[15px] text-muted">{unit}</span>
		</span>
	</div>
{/snippet}
