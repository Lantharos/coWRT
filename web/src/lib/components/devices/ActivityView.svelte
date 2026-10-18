<script lang="ts">
	import { api } from '#lib/api/client.js';
	import type { Activity } from '#lib/api/types.js';
	import { bytes, joined } from '#lib/format.js';
	import Bars from '#lib/components/charts/Bars.svelte';

	const EVERY = 30000;
	const SHOWN = 8;

	let { source }: { source: string } = $props();

	let activity = $state<Activity | null>(null);

	$effect(() => {
		const path = source;
		const load = () =>
			api
				.get<Activity>(path)
				.then((next) => (activity = next))
				.catch(() => {});
		load();
		const timer = setInterval(() => document.visibilityState === 'visible' && load(), EVERY);
		return () => clearInterval(timer);
	});

	const totals = $derived(activity?.days.map((d) => d.down + d.up) ?? []);
	const month = $derived(joined(bytes(totals.reduce((sum, value) => sum + value, 0))));
</script>

{#if activity}
	<div class="flex flex-col gap-8">
		<div class="flex flex-col gap-3">
			<div class="flex items-baseline justify-between text-[15px]">
				<h3>Last 30 days</h3>
				<span class="text-[14px] text-muted">{month}</span>
			</div>
			<Bars values={totals} />
		</div>

		<div class="flex flex-col">
			<div class="flex items-baseline justify-between pb-1 text-[15px]">
				<h3>Sites today</h3>
				{#if activity.today.blocked}
					<span class="text-[14px] text-muted">{activity.today.blocked} blocked</span>
				{/if}
			</div>
			{#each activity.sites.slice(0, SHOWN) as site (site.domain)}
				<div class="flex items-center justify-between gap-4 py-1.5 text-[14px]">
					<span class="truncate">{site.domain}</span>
					<span class="shrink-0 {site.blocked ? 'text-danger' : 'text-muted'}">
						{site.blocked ? 'Blocked' : `${site.lookups}×`}
					</span>
				</div>
			{:else}
				<p class="py-1.5 text-[14px] text-muted">Nothing yet today.</p>
			{/each}
		</div>
	</div>
{/if}
