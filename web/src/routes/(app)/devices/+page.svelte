<script lang="ts">
	import { Search } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { Device, NetworksOverview } from '#lib/api/types.js';
	import DeviceRow from '#lib/components/devices/DeviceRow.svelte';
	import DeviceSheet from '#lib/components/devices/DeviceSheet.svelte';
	import { networkColors } from '#lib/components/networks/colors.js';
	import List from '#lib/components/ui/List.svelte';
	import PageHeader from '#lib/components/ui/PageHeader.svelte';

	const devices = poll(() => api.get<Device[]>('/devices'), 5000);
	const networks = poll(() => api.get<NetworksOverview>('/networks'), 30000);

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

	const groups = $derived.by(() => {
		const online = matches.filter((d) => d.online);
		const known = networks.data?.networks ?? [];
		const byNetwork = known
			.map((network) => ({
				id: network.id,
				label: network.name,
				color: networkColors[network.color] as string | null,
				items: online.filter((d) => d.network === network.id)
			}))
			.filter((group) => group.items.length);
		const elsewhere = online.filter((d) => !known.some((n) => n.id === d.network));
		const offline = matches.filter((d) => !d.online);
		return [
			...byNetwork,
			...(elsewhere.length ? [{ id: 'other', label: 'Other', color: null, items: elsewhere }] : []),
			...(offline.length
				? [{ id: 'offline', label: 'Seen before', color: null, items: offline }]
				: [])
		];
	});

	const online = $derived(devices.data?.filter((d) => d.online).length);
	const offline = $derived(devices.data?.filter((d) => !d.online).length ?? 0);
</script>

<PageHeader detail={offline ? `${offline} more seen recently.` : undefined}>
	{#snippet headline()}
		{online ?? '–'}
		{online === 1 ? 'device' : 'devices'} online
	{/snippet}
	{#snippet action()}
		<label
			class="flex h-11 w-full items-center gap-2.5 rounded-control bg-surface px-3.5 text-muted focus-within:bg-raised sm:w-72"
		>
			<Search size={17} class="shrink-0" />
			<input
				bind:value={query}
				placeholder="Find a device"
				class="w-full bg-transparent text-[15px] text-fg outline-none placeholder:text-faint"
			/>
		</label>
	{/snippet}
</PageHeader>

<div class="flex max-w-[860px] flex-col gap-8">
	{#each groups as group (group.id)}
		<List>
			{#snippet aside()}
				<span class="flex items-center gap-2 text-[15px] text-muted">
					{#if group.color}<span class="size-2 rounded-full" style:background={group.color}
						></span>{/if}
					{group.label}
				</span>
			{/snippet}
			{#each group.items as device (device.mac)}
				<DeviceRow {device} onselect={(d) => (selected = d.mac)} />
			{/each}
		</List>
	{:else}
		<p class="px-3 text-muted">{devices.data ? 'No devices match.' : 'Looking around…'}</p>
	{/each}
</div>

<DeviceSheet
	device={devices.data?.find((device) => device.mac === selected) ?? null}
	onclose={() => (selected = null)}
	onchange={() => devices.refresh()}
/>
