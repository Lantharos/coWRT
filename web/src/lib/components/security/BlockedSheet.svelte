<script lang="ts">
	import { api } from '#lib/api/client.js';
	import type { ProtectionActivity } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Bars from '#lib/components/charts/Bars.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';
	import Setting from '#lib/components/ui/Setting.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import { timeline } from './timeline';

	type Props = {
		activity: ProtectionActivity | null;
		open: boolean;
		onclose: () => void;
		onsaved: () => void;
	};
	let { activity, open, onclose, onsaved }: Props = $props();

	let busy = $state(false);

	const blocked = $derived(activity?.buckets.reduce((sum, b) => sum + b.blocked, 0) ?? 0);
	const lookups = $derived(activity?.buckets.reduce((sum, b) => sum + b.lookups, 0) ?? 0);

	async function setTracking(enabled: boolean) {
		busy = true;
		try {
			await api.put('/protection/activity', { enabled });
			toasts.show(enabled ? 'Keeping track of sites' : 'No longer keeping track of sites');
			onsaved();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<Sheet {open} title="Blocked today" {onclose}>
	{#if activity}
		<div class="flex flex-col gap-8">
			{#if activity.enabled}
				<div class="flex flex-col gap-3">
					<p class="flex items-baseline gap-2 font-display">
						<span class="text-4xl">{blocked.toLocaleString()}</span>
						<span class="text-[15px] text-muted">of {lookups.toLocaleString()} lookups</span>
					</p>
					<Bars
						values={timeline(activity.buckets, 48, 3, (b) => b.blocked)}
						height={72}
						highlightLast={false}
					/>
					<p class="text-[13px] text-faint">Last 24 hours</p>
				</div>

				<div class="flex flex-col">
					<h3 class="pb-1 text-[15px]">Blocked most</h3>
					{#each activity.topBlocked as site (site.domain)}
						<div class="flex items-center justify-between gap-4 py-1.5 text-[14px]">
							<span class="truncate">{site.domain}</span>
							<span class="shrink-0 text-muted">{site.blocked}×</span>
						</div>
					{:else}
						<p class="py-1.5 text-[14px] text-muted">Nothing blocked yet today.</p>
					{/each}
				</div>
			{/if}

			<Setting title="Keep track of sites" hint="Stays on the router and clears every night">
				<Toggle
					checked={activity.enabled}
					disabled={busy}
					label="Keep track of sites"
					onchange={setTracking}
				/>
			</Setting>
		</div>
	{/if}
</Sheet>
