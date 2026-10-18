<script lang="ts">
	import { Plus, Search } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { opened } from '#lib/opened.svelte.js';
	import { devices } from '#lib/api/devices.svelte.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { NetworksOverview, Person } from '#lib/api/types.js';
	import DeviceRow from '#lib/components/devices/DeviceRow.svelte';
	import DeviceSheet from '#lib/components/devices/DeviceSheet.svelte';
	import { palette } from '#lib/palette.js';
	import PersonRow from '#lib/components/devices/PersonRow.svelte';
	import PersonSheet from '#lib/components/devices/PersonSheet.svelte';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import List from '#lib/components/ui/List.svelte';
	import PageHeader from '#lib/components/ui/PageHeader.svelte';

	devices.use();
	const networks = poll(() => api.get<NetworksOverview>('/networks'), 30000);
	const people = poll(() => api.get<Person[]>('/people'), 30000);

	let query = $state('');
	let selected = $state<string | null>(null);
	let personSelected = $state<string | 'new' | null>(null);

	opened('device', (mac) => (selected = mac));
	opened('person', (id) => (personSelected = id));

	const everyone = $derived(people.data ?? []);
	const ownerOf = (mac: string) => {
		const owner = all.find((d) => d.mac === mac)?.owner;
		return everyone.find((p) => p.id === owner);
	};

	const all = $derived(devices.list ?? []);
	const matches = $derived.by(() => {
		const needle = query.trim().toLowerCase();
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
				color: palette[network.color] as string | undefined,
				items: online.filter((d) => d.network === network.id)
			}))
			.filter((group) => group.items.length);
		const elsewhere = online.filter((d) => !known.some((n) => n.id === d.network));
		const offline = matches.filter((d) => !d.online);
		return [
			...byNetwork,
			...(elsewhere.length
				? [{ id: 'other', label: 'Other', color: undefined, items: elsewhere }]
				: []),
			...(offline.length
				? [{ id: 'offline', label: 'Seen before', color: undefined, items: offline }]
				: [])
		];
	});

	const online = $derived(all.filter((d) => d.online).length);
	const offline = $derived(all.length - online);
</script>

<PageHeader detail={offline ? `And ${offline} more seen recently.` : undefined}>
	{#snippet headline()}
		{devices.list ? online : '–'}
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
	{#if !query.trim()}
		<List label="People">
			{#snippet aside()}
				<Button size="sm" variant="ghost" onclick={() => (personSelected = 'new')}>
					<Plus size={15} /> Add
				</Button>
			{/snippet}
			{#each everyone as person (person.id)}
				<PersonRow
					{person}
					owned={all.filter((d) => d.owner === person.id)}
					onselect={() => (personSelected = person.id)}
				/>
			{:else}
				<p class="px-3 pb-2 text-[14px] text-muted">
					Group devices by who uses them to pause or set bedtimes for all of them at once.
				</p>
			{/each}
		</List>
	{/if}

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
				<DeviceRow
					{device}
					owner={ownerOf(device.mac)}
					tint={group.color}
					onselect={(d) => (selected = d.mac)}
				/>
			{/each}
		</List>
	{:else}
		<p class="px-3 text-muted">{devices.list ? 'No devices match.' : 'Looking around…'}</p>
	{/each}
</div>

<PersonSheet
	person={personSelected === 'new'
		? 'new'
		: (everyone.find((p) => p.id === personSelected) ?? null)}
	devices={all}
	onclose={() => (personSelected = null)}
	onsaved={async (id) => {
		await people.refresh();
		personSelected = everyone.some((p) => p.id === id) ? id : null;
	}}
	ondevices={() => devices.refresh()}
/>

<DeviceSheet
	people={everyone}
	device={all.find((device) => device.mac === selected) ?? null}
	onclose={() => (selected = null)}
	onchange={() => devices.refresh()}
/>
