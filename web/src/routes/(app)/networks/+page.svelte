<script lang="ts">
	import { Plus } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { live } from '#lib/api/live.svelte.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type {
		Device,
		InternetOverview,
		Network,
		NetworksOverview,
		Port
	} from '#lib/api/types.js';
	import InternetSheet from '#lib/components/internet/InternetSheet.svelte';
	import NetworkSheet from '#lib/components/networks/NetworkSheet.svelte';
	import PortSheet from '#lib/components/networks/PortSheet.svelte';
	import PortStrip from '#lib/components/networks/PortStrip.svelte';
	import { networkColors } from '#lib/components/networks/colors.js';
	import Button from '#lib/components/ui/Button.svelte';
	import List from '#lib/components/ui/List.svelte';
	import PageHeader from '#lib/components/ui/PageHeader.svelte';
	import Row from '#lib/components/ui/Row.svelte';

	const overview = poll(() => api.get<NetworksOverview>('/networks'), 10000);
	const settings = poll(() => api.get<InternetOverview>('/internet'), 60000);
	const devices = poll(() => api.get<Device[]>('/devices'), 15000);

	let editing = $state<string | 'new' | null>(null);
	let selectedPort = $state<string | null>(null);
	let internetOpen = $state(false);

	const networks = $derived(overview.data?.networks ?? []);
	const ports = $derived(overview.data?.ports ?? []);
	const snapshot = $derived(live.snapshot);
	const editingNetwork = $derived(
		editing === 'new' ? 'new' : (networks.find((n) => n.id === editing) ?? null)
	);

	const names = (items: string[]) =>
		items.length < 2 ? (items[0] ?? '') : `${items.slice(0, -1).join(', ')} and ${items.at(-1)}`;

	function detail(network: Network) {
		const count = devices.data?.filter((d) => d.online && d.network === network.id).length ?? 0;
		const parts = [
			network.address?.split('/')[0] ?? 'No address',
			`${count} ${count === 1 ? 'device' : 'devices'}`
		];
		if (network.wifi.length) parts.push(`${network.wifi.join(', ')} Wi-Fi`);
		if (!network.internet) parts.push('no internet');
		else if (!network.trusted) parts.push('kept apart');
		return parts.join(' · ');
	}

	const internet = $derived.by(() => {
		const wan = snapshot?.wan;
		const online = snapshot?.internet.online;
		const kind = { dhcp: 'Automatic', static: 'Fixed address', pppoe: 'PPPoE' }[
			settings.data?.connection.kind ?? 'dhcp'
		];
		return {
			detail: [kind, wan?.ipv4?.split('/')[0]].filter(Boolean).join(' · '),
			value: !snapshot ? null : online ? `${snapshot.internet.latencyMs ?? '–'} ms` : 'Offline',
			tone: (online ? 'muted' : 'bad') as 'muted' | 'bad'
		};
	});
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
			<Row title={network.name} detail={detail(network)} onclick={() => (editing = network.id)}>
				{#snippet leading()}
					<span class="size-3 rounded-full" style:background={networkColors[network.color]}></span>
				{/snippet}
			</Row>
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

	<List>
		<Row
			title="Internet"
			detail={internet.detail}
			value={internet.value}
			tone={internet.tone}
			onclick={() => (internetOpen = true)}
		/>
	</List>
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

<InternetSheet
	settings={settings.data}
	open={internetOpen}
	onclose={() => (internetOpen = false)}
	onsaved={() => settings.refresh()}
/>
