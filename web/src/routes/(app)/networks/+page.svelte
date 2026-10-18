<script lang="ts">
	import { House, Network as NetworkIcon, Plus, Signpost, Tags } from '@lucide/svelte';
	import { devices } from '#lib/api/devices.svelte.js';
	import { bits, joined } from '#lib/format.js';
	import { api } from '#lib/api/client.js';
	import { opened } from '#lib/opened.svelte.js';
	import { live } from '#lib/api/live.svelte.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { LanOverview, Network, NetworksOverview, Port } from '#lib/api/types.js';
	import InternetList from '#lib/components/internet/InternetList.svelte';
	import NamesSheet from '#lib/components/networks/NamesSheet.svelte';
	import RoutesSheet from '#lib/components/networks/RoutesSheet.svelte';
	import NetworkSheet from '#lib/components/networks/NetworkSheet.svelte';
	import PortSheet from '#lib/components/networks/PortSheet.svelte';
	import PortStrip from '#lib/components/networks/PortStrip.svelte';
	import { mainPrefix } from '#lib/ipv6.js';
	import { palette } from '#lib/palette.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import List from '#lib/components/ui/List.svelte';
	import PageHeader from '#lib/components/ui/PageHeader.svelte';
	import Row from '#lib/components/ui/Row.svelte';

	const overview = poll(() => api.get<NetworksOverview>('/networks'), 10000);
	const lan = poll(() => api.get<LanOverview>('/lan'), 60000);
	devices.use();

	let editing = $state<string | 'new' | null>(null);
	let selectedPort = $state<string | null>(null);
	let speedOpen = $state(false);
	let lanPanel = $state<'names' | 'routes' | null>(null);

	opened('network', (id) => (editing = id));
	opened('open', (panel) => {
		if (panel === 'speed') speedOpen = true;
		if (panel === 'names' || panel === 'routes') lanPanel = panel;
	});

	const count = (n: number, one: string, many: string) =>
		n ? `${n} ${n === 1 ? one : many}` : 'None';

	const networks = $derived(overview.data?.networks ?? []);
	const ports = $derived(overview.data?.ports ?? []);
	const snapshot = $derived(live.snapshot);
	const editingNetwork = $derived(
		editing === 'new' ? 'new' : (networks.find((n) => n.id === editing) ?? null)
	);

	const members = (network: Network) =>
		(devices.list ?? []).filter((d) => d.online && d.network === network.id);
	const rate = (network: Network) =>
		members(network).reduce((sum, d) => sum + d.usage.down + d.usage.up, 0);

	const names = (items: string[]) =>
		items.length < 2 ? (items[0] ?? '') : `${items.slice(0, -1).join(', ')} and ${items.at(-1)}`;

	function detail(network: Network) {
		const count = members(network).length;
		const prefix = network.ipv6.enabled ? mainPrefix(network.ipv6.prefixes) : null;
		const parts = [
			network.address?.split('/')[0] ?? 'No address',
			...(prefix ? [prefix] : []),
			`${count} ${count === 1 ? 'device' : 'devices'}`
		];
		if (network.wifi.length) parts.push(`${network.wifi.join(', ')} Wi-Fi`);
		if (!network.internet) parts.push('no internet');
		else if (!network.trusted) parts.push('kept apart');
		return parts.join(' · ');
	}
</script>

<PageHeader
	detail={networks.length
		? `${names(networks.map((n) => n.name))}, across ${ports.length} ports.`
		: undefined}
>
	{#snippet headline()}
		{networks.length || '–'}
		{networks.length === 1 ? 'network' : 'networks'}
	{/snippet}
	{#snippet action()}
		<Button variant="primary" onclick={() => (editing = 'new')}
			><Plus size={17} /> New network</Button
		>
	{/snippet}
</PageHeader>

<div class="flex max-w-[860px] flex-col gap-10">
	<List>
		{#each networks as network (network.id)}
			<Row
				title={network.name}
				detail={detail(network)}
				icon={network.primary ? House : NetworkIcon}
				tint={palette[network.color]}
				graph={devices.traffic(members(network).map((d) => d.mac))}
				value={joined(bits(rate(network)))}
				onclick={() => (editing = network.id)}
			/>
		{/each}
	</List>

	{#if ports.length}
		<List label="Ports">
			<PortStrip
				{ports}
				{networks}
				wan={snapshot?.wan.device ?? null}
				onselect={(port: Port) => (selectedPort = port.name)}
			/>
		</List>
	{/if}

	<InternetList bind:speedOpen />

	{#if lan.data}
		<List>
			<Row
				title="Local names"
				detail="Like nas.{lan.data.domain ?? 'lan'}"
				icon={Tags}
				value={count(lan.data.names.length, 'name', 'names')}
				onclick={() => (lanPanel = 'names')}
			/>
			<Row
				title="Routes"
				detail="Reach networks behind other routers"
				icon={Signpost}
				value={count(lan.data.routes.length, 'route', 'routes')}
				onclick={() => (lanPanel = 'routes')}
			/>
		</List>
	{/if}
</div>

<NetworkSheet
	network={editingNetwork}
	{networks}
	onclose={() => (editing = null)}
	onsaved={() => overview.refresh()}
/>

<PortSheet
	port={ports.find((p) => p.name === selectedPort) ?? null}
	{networks}
	onclose={() => (selectedPort = null)}
	onsaved={() => overview.refresh()}
/>

<NamesSheet
	names={lan.data?.names ?? []}
	domain={lan.data?.domain ?? null}
	devices={devices.list ?? []}
	open={lanPanel === 'names'}
	onclose={() => (lanPanel = null)}
	onsaved={() => lan.refresh()}
/>
<RoutesSheet
	routes={lan.data?.routes ?? []}
	{networks}
	open={lanPanel === 'routes'}
	onclose={() => (lanPanel = null)}
	onsaved={() => lan.refresh()}
/>
