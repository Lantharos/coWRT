<script lang="ts">
	import { Plus, Radio as RadioIcon, Wifi, WifiOff } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { devices } from '#lib/api/devices.svelte.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type {
		Device,
		NetworksOverview,
		Radio,
		WifiNetwork,
		WifiOverview
	} from '#lib/api/types.js';
	import { bandLabel } from '#lib/format.js';
	import { toasts } from '#lib/toast.svelte.js';
	import { networkColors } from '#lib/components/networks/colors.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import List from '#lib/components/ui/List.svelte';
	import PageHeader from '#lib/components/ui/PageHeader.svelte';
	import Row from '#lib/components/ui/Row.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';
	import NetworkSheet from '#lib/components/wifi/NetworkSheet.svelte';
	import RadioSheet from '#lib/components/wifi/RadioSheet.svelte';

	const wifi = poll(() => api.get<WifiOverview>('/wifi'), 10000);
	const networks = poll(() => api.get<NetworksOverview>('/networks'), 30000);
	devices.use();

	let editing = $state<string | 'new' | null>(null);
	let tuning = $state<string | null>(null);

	const securityLabel: Record<string, string> = {
		wpa3: 'WPA3',
		'wpa2-wpa3': 'WPA2 + WPA3',
		wpa2: 'WPA2',
		'enhanced-open': 'Open, encrypted',
		open: 'Open'
	};

	const radios = $derived(wifi.data?.radios ?? []);
	const wifiNetworks = $derived(wifi.data?.networks ?? []);
	const you = $derived(devices.list?.find((device) => device.isYou));
	const onWifi = (
		match: (connection: Extract<Device['connection'], { kind: 'wifi' }>) => boolean
	) =>
		(devices.list ?? []).filter(
			(device) => device.connection.kind === 'wifi' && match(device.connection)
		);
	const tintOf = (network: WifiNetwork) => {
		const color = networks.data?.networks.find((n) => n.id === network.network)?.color;
		return color ? networkColors[color] : undefined;
	};
	const clients = $derived(wifiNetworks.reduce((sum, network) => sum + network.clients, 0));
	const editingNetwork = $derived(
		editing === 'new'
			? 'new'
			: (wifi.data?.networks.find((network) => network.id === editing) ?? null)
	);

	async function setEnabled(network: WifiNetwork, enabled: boolean) {
		try {
			await api.put(`/wifi/networks/${network.id}`, { ...network, enabled });
			toasts.show(enabled ? `${network.ssid} is back on` : `${network.ssid} turned off`);
		} catch (err) {
			toasts.error(err);
		}
		wifi.refresh();
	}

	function networkDetail(network: WifiNetwork) {
		const parts = [
			network.bands.map((band) => bandLabel[band]).join(' & '),
			securityLabel[network.security]
		];
		if (network.hidden) parts.push('hidden');
		return parts.join(' · ');
	}

	function radioDetail(radio: Radio) {
		if (!radio.enabled) return 'Turned off';
		const channel = radio.live ? `Channel ${radio.live.channel}` : 'Starting';
		const width = radio.htmode?.match(/\d+$/)?.[0];
		return `${channel}${radio.channel === 'auto' ? ' (auto)' : ''}${width ? ` · ${width} MHz` : ''}`;
	}

	const count = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
</script>

<PageHeader
	detail={wifi.data && wifiNetworks.length
		? `${count(clients, 'device', 'devices')} connected right now.`
		: undefined}
>
	{#snippet headline()}
		{#if !wifi.data}
			Wi-Fi
		{:else if wifiNetworks.length}
			{count(wifiNetworks.length, 'Wi-Fi network', 'Wi-Fi networks')}
		{:else}
			No Wi-Fi yet
		{/if}
	{/snippet}
	{#snippet action()}
		<Button variant="primary" onclick={() => (editing = 'new')}
			><Plus size={17} /> New network</Button
		>
	{/snippet}
</PageHeader>

<div class="flex max-w-[860px] flex-col gap-10">
	<List>
		{#each wifiNetworks as network (network.id)}
			<Row
				title={network.ssid}
				detail={networkDetail(network)}
				icon={network.enabled ? Wifi : WifiOff}
				tint={tintOf(network)}
				dimmed={!network.enabled}
				graph={network.enabled
					? devices.traffic(onWifi((c) => c.ssid === network.ssid).map((d) => d.mac))
					: undefined}
				value={network.enabled ? count(network.clients, 'device', 'devices') : null}
				onclick={() => (editing = network.id)}
			>
				{#snippet trailing()}
					<Toggle
						checked={network.enabled}
						label="Turn {network.ssid} on or off"
						onchange={(on) => setEnabled(network, on)}
					/>
				{/snippet}
			</Row>
		{/each}
	</List>

	<List label="Radios">
		{#each radios as radio (radio.name)}
			<Row
				title={bandLabel[radio.band]}
				detail={radioDetail(radio)}
				icon={RadioIcon}
				tint={radio.enabled && radio.up ? 'var(--color-accent)' : undefined}
				dimmed={!radio.enabled}
				graph={radio.enabled
					? devices.traffic(onWifi((c) => c.band === radio.band).map((d) => d.mac))
					: undefined}
				value={radio.live?.utilization != null ? `${radio.live.utilization}% busy` : null}
				onclick={() => (tuning = radio.name)}
			/>
		{/each}
	</List>
</div>

<NetworkSheet
	network={editingNetwork}
	{radios}
	networks={networks.data?.networks ?? []}
	connectedHere={!!you &&
		you.connection.kind === 'wifi' &&
		editingNetwork !== 'new' &&
		you.connection.ssid === editingNetwork?.ssid}
	onclose={() => (editing = null)}
	onsaved={() => wifi.refresh()}
/>

<RadioSheet
	radio={wifi.data?.radios.find((radio) => radio.name === tuning) ?? null}
	onclose={() => (tuning = null)}
	onsaved={() => wifi.refresh()}
/>
