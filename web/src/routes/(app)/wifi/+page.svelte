<script lang="ts">
	import { Plus, Radio as RadioIcon } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { Device, Radio, WifiNetwork, WifiOverview } from '#lib/api/types.js';
	import { bandLabel } from '#lib/format.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Block from '#lib/components/ui/Block.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Section from '#lib/components/ui/Section.svelte';
	import Toggle from '#lib/components/ui/Toggle.svelte';
	import NetworkSheet from '#lib/components/wifi/NetworkSheet.svelte';
	import RadioSheet from '#lib/components/wifi/RadioSheet.svelte';

	const wifi = poll(() => api.get<WifiOverview>('/wifi'), 10000);
	const devices = poll(() => api.get<Device[]>('/devices'), 10000);

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
	const you = $derived(devices.data?.find((device) => device.isYou));
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

	function radioSummary(radio: Radio) {
		if (!radio.enabled) return 'Turned off';
		const channel = radio.live ? `Channel ${radio.live.channel}` : 'Starting';
		const width = radio.htmode?.match(/\d+$/)?.[0];
		return `${channel}${radio.channel === 'auto' ? ' (auto)' : ''}${width ? ` · ${width} MHz` : ''}`;
	}
</script>

<div class="flex items-end justify-between gap-6">
	<div>
		<h1 class="font-display text-[42px] leading-none font-medium">Wi-Fi</h1>
		<p class="mt-2 text-muted">Your networks and the radios that carry them.</p>
	</div>
	<Button variant="primary" onclick={() => (editing = 'new')}><Plus size={17} /> New network</Button
	>
</div>

<Section title="Networks">
	{#each wifi.data?.networks ?? [] as network (network.id)}
		<div class="flex items-center gap-4 bg-surface pr-6 transition-colors hover:bg-raised">
			<button
				type="button"
				onclick={() => (editing = network.id)}
				class="flex min-w-0 flex-1 cursor-pointer flex-col py-4 pl-6 text-left"
			>
				<span class="truncate text-[17px] font-medium {network.enabled ? '' : 'text-muted'}"
					>{network.ssid}</span
				>
				<span class="truncate text-[13px] text-muted">
					{network.bands.map((band) => bandLabel[band]).join(' & ')} · {securityLabel[
						network.security
					]} · {network.clients}
					{network.clients === 1 ? 'device' : 'devices'}{network.hidden ? ' · hidden' : ''}
				</span>
			</button>
			<Toggle
				checked={network.enabled}
				label="Turn {network.ssid} on or off"
				onchange={(on) => setEnabled(network, on)}
			/>
		</div>
	{:else}
		<Block description={wifi.data ? 'No networks yet.' : 'Loading networks…'} />
	{/each}
</Section>

<Section title="Radios">
	<div class="grid gap-[3px] sm:grid-cols-2">
		{#each radios as radio (radio.name)}
			<button
				type="button"
				onclick={() => (tuning = radio.name)}
				class="flex cursor-pointer flex-col gap-5 bg-surface px-6 py-5 text-left transition-colors hover:bg-raised"
			>
				<span class="flex items-center justify-between">
					<span class="font-display text-2xl">{bandLabel[radio.band]}</span>
					<RadioIcon size={20} class={radio.enabled && radio.up ? 'text-accent' : 'text-faint'} />
				</span>
				<span class="flex flex-col gap-1">
					<span class="text-[15px]">{radioSummary(radio)}</span>
					{#if radio.live?.utilization != null}
						<span class="flex items-center gap-3 text-[13px] text-muted">
							<span class="h-1.5 flex-1 overflow-hidden rounded-full bg-overlay">
								<span
									class="block h-full rounded-full bg-accent"
									style:width="{radio.live.utilization}%"
								></span>
							</span>
							{radio.live.utilization}% airtime busy
						</span>
					{/if}
				</span>
			</button>
		{/each}
	</div>
</Section>

<NetworkSheet
	network={editingNetwork}
	{radios}
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
