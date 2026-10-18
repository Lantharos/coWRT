<script lang="ts">
	import { ArrowRightLeft, AtSign, Gamepad2, KeyRound, Plus, Route } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type {
		Device,
		Forward,
		NetworksOverview,
		SharingOverview,
		VpnOverview
	} from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import AddressSheet from '#lib/components/access/AddressSheet.svelte';
	import ForwardSheet from '#lib/components/access/ForwardSheet.svelte';
	import ProviderSheet from '#lib/components/access/ProviderSheet.svelte';
	import VpnSheet from '#lib/components/access/VpnSheet.svelte';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import List from '#lib/components/ui/List.svelte';
	import PageHeader from '#lib/components/ui/PageHeader.svelte';
	import Row from '#lib/components/ui/Row.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';

	type Panel = 'vpn' | 'provider' | 'address';

	const sharing = poll(() => api.get<SharingOverview>('/sharing'), 30000);
	const devices = poll(() => api.get<Device[]>('/devices'), 30000);
	const vpn = poll(() => api.get<VpnOverview>('/vpn'), 15000);
	const networks = poll(() => api.get<NetworksOverview>('/networks'), 60000);

	let editing = $state<string | 'new' | null>(null);
	let panel = $state<Panel | null>(null);
	let testing = $state(false);
	let reachable = $state<boolean | null>(null);

	const reach = $derived(sharing.data?.reach);
	const forwards = $derived(sharing.data?.forwards ?? []);
	const deviceName = (mac: string) => devices.data?.find((d) => d.mac === mac)?.name ?? mac;
	const editingForward = $derived(
		editing === 'new' ? 'new' : (forwards.find((f) => f.id === editing) ?? null)
	);
	const protocolLabel = { tcp: 'TCP', udp: 'UDP', both: 'TCP and UDP' };

	const on = 'var(--color-accent)';
	const connected = $derived(
		(vpn.data?.server.peers ?? []).filter(
			(p) => p.status.handshake && Date.now() / 1000 - p.status.handshake < 180
		).length
	);

	const open = $derived(reachable ?? (reach ? !reach.behindNat : null));
	const vpnValue = $derived.by(() => {
		const server = vpn.data?.server;
		if (!server?.enabled) return 'Off';
		if (connected) return `${connected} connected`;
		return `${server.peers.length} ${server.peers.length === 1 ? 'device' : 'devices'}`;
	});
	const providerValue = $derived.by(() => {
		const client = vpn.data?.client;
		if (!client?.enabled) return 'Off';
		const names = (networks.data?.networks ?? [])
			.filter((n) => client.networks.includes(n.id))
			.map((n) => n.name);
		return names.length ? `${client.name} · ${names.join(', ')}` : client.name;
	});

	async function test() {
		testing = true;
		try {
			reachable = (await api.post<{ open: boolean }>('/forwards/check', {})).open;
		} catch (err) {
			toasts.error(err);
		} finally {
			testing = false;
		}
	}

	async function setForward(forward: Forward, enabled: boolean) {
		try {
			await api.put(`/forwards/${forward.id}`, { ...forward, enabled });
		} catch (err) {
			toasts.error(err);
		}
		sharing.refresh();
	}

	async function setUpnp(enabled: boolean) {
		if (enabled && !sharing.data?.upnp.installed) toasts.show('Installing, this takes a moment');
		try {
			await api.put('/upnp', { enabled });
		} catch (err) {
			toasts.error(err);
		}
		sharing.refresh();
	}
</script>

<PageHeader
	detail={!reach
		? undefined
		: open
			? `Your public address is ${reach.publicIp}.`
			: "There's another router in front of this one, so port forwards and the VPN only work from inside."}
>
	{#snippet headline()}
		{#if !reach}
			<span class="text-muted">Checking how the internet sees you</span>
		{:else if open}
			Home is <span class="text-accent">reachable</span> from outside
		{:else}
			Home <span class="text-danger">can't be reached</span> from outside
		{/if}
	{/snippet}
	{#snippet action()}
		<Button onclick={test} busy={testing}>Test again</Button>
	{/snippet}
</PageHeader>

<div class="flex max-w-[860px] flex-col gap-10">
	<List>
		<Row
			title="VPN"
			icon={KeyRound}
			tint={vpn.data?.server.enabled ? on : undefined}
			detail="Reach home from your phone and laptop"
			value={vpnValue}
			onclick={() => (panel = 'vpn')}
		/>
		<Row
			title="VPN provider"
			icon={Route}
			tint={vpn.data?.client.enabled ? on : undefined}
			detail="Send whole networks through a VPN"
			value={providerValue}
			onclick={() => (panel = 'provider')}
		/>
		<Row
			title="Your own address"
			icon={AtSign}
			tint={sharing.data?.ddns.enabled ? on : undefined}
			detail="A hostname that always points home"
			value={sharing.data?.ddns.enabled ? sharing.data.ddns.hostname : 'Off'}
			onclick={() => (panel = 'address')}
		/>
	</List>

	<List label="Port forwards">
		{#snippet aside()}
			<Button size="sm" variant="ghost" onclick={() => (editing = 'new')}>
				<Plus size={15} /> Add
			</Button>
		{/snippet}
		{#each forwards as forward (forward.id)}
			<Row
				title={forward.name}
				icon={ArrowRightLeft}
				tint={forward.enabled ? on : undefined}
				detail="{protocolLabel[forward.protocol]} {forward.external} to {deviceName(forward.mac)}"
				dimmed={!forward.enabled}
				onclick={() => (editing = forward.id)}
			>
				{#snippet trailing()}
					<Toggle
						checked={forward.enabled}
						label="Open {forward.name}"
						onchange={(on) => setForward(forward, on)}
					/>
				{/snippet}
			</Row>
		{/each}

		{#if sharing.data}
			<Row
				title="Let apps open ports"
				icon={Gamepad2}
				tint={sharing.data.upnp.enabled ? on : undefined}
				detail={sharing.data.upnp.enabled && sharing.data.upnp.mappings.length
					? `${sharing.data.upnp.mappings.length} open right now`
					: 'UPnP and NAT-PMP, for games and consoles'}
			>
				{#snippet trailing()}
					<Toggle
						checked={sharing.data!.upnp.enabled}
						label="Let apps open ports"
						onchange={setUpnp}
					/>
				{/snippet}
			</Row>
		{/if}
	</List>
</div>

<ForwardSheet
	forward={editingForward}
	devices={devices.data ?? []}
	onclose={() => (editing = null)}
	onsaved={() => sharing.refresh()}
/>
<VpnSheet
	server={vpn.data?.server ?? null}
	open={panel === 'vpn'}
	onclose={() => (panel = null)}
	onsaved={() => vpn.refresh()}
/>
<ProviderSheet
	provider={vpn.data?.client ?? null}
	networks={networks.data?.networks ?? []}
	open={panel === 'provider'}
	onclose={() => (panel = null)}
	onsaved={() => vpn.refresh()}
/>
<AddressSheet
	ddns={sharing.data?.ddns ?? null}
	open={panel === 'address'}
	onclose={() => (panel = null)}
	onsaved={() => sharing.refresh()}
/>
