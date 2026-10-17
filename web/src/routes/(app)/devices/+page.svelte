<script lang="ts">
	import { Search } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { Device } from '#lib/api/types.js';
	import DeviceRow from '#lib/components/devices/DeviceRow.svelte';
	import DeviceSheet from '#lib/components/devices/DeviceSheet.svelte';
	import Block from '#lib/components/ui/Block.svelte';
	import Section from '#lib/components/ui/Section.svelte';

	const devices = poll(() => api.get<Device[]>('/devices'), 5000);

	let query = $state('');
	let selected = $state<string | null>(null);

	const matches = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		const all = devices.data ?? [];
		if (!needle) return all;
		return all.filter((device) =>
			[device.name, device.hostname, device.ipv4, device.mac].some((field) =>
				field?.toLowerCase().includes(needle)
			)
		);
	});
	const online = $derived(matches.filter((device) => device.online));
	const offline = $derived(matches.filter((device) => !device.online));
</script>

<div class="flex flex-col gap-6 sm:flex-row sm:items-end sm:justify-between">
	<div>
		<h1 class="font-display text-[42px] leading-none font-medium">Devices</h1>
		<p class="mt-2 text-muted">
			{#if devices.data}
				{devices.data.filter((d) => d.online).length} online, {devices.data.length} known
			{:else}
				Looking around your network…
			{/if}
		</p>
	</div>
	<label
		class="flex h-11 w-full items-center gap-2.5 rounded-control bg-raised px-3.5 text-muted focus-within:bg-overlay sm:w-72"
	>
		<Search size={17} class="shrink-0" />
		<input
			bind:value={query}
			placeholder="Search by name, IP or MAC"
			class="w-full bg-transparent text-[15px] text-fg outline-none placeholder:text-faint"
		/>
	</label>
</div>

<Section title="Online">
	{#each online as device (device.mac)}
		<DeviceRow {device} onselect={(d) => (selected = d.mac)} />
	{:else}
		<Block description={devices.data ? 'No devices match.' : 'Looking for devices…'} />
	{/each}
</Section>

{#if offline.length}
	<Section title="Seen before">
		{#each offline as device (device.mac)}
			<DeviceRow {device} onselect={(d) => (selected = d.mac)} />
		{/each}
	</Section>
{/if}

<DeviceSheet
	device={devices.data?.find((device) => device.mac === selected) ?? null}
	onclose={() => (selected = null)}
	onchange={() => devices.refresh()}
/>
