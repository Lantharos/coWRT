<script lang="ts">
	import { Plus } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { live } from '#lib/api/live.svelte.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { Network, NetworksOverview, Port } from '#lib/api/types.js';
	import { duration } from '#lib/format.js';
	import NetworkSheet from '#lib/components/networks/NetworkSheet.svelte';
	import PortSheet from '#lib/components/networks/PortSheet.svelte';
	import PortTile from '#lib/components/networks/PortTile.svelte';
	import { networkColors } from '#lib/components/networks/colors.js';
	import Block from '#lib/components/ui/Block.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Facts from '#lib/components/ui/Facts.svelte';
	import Section from '#lib/components/ui/Section.svelte';

	const overview = poll(() => api.get<NetworksOverview>('/networks'), 10000);

	let editing = $state<string | 'new' | null>(null);
	let selectedPort = $state<string | null>(null);

	const networks = $derived(overview.data?.networks ?? []);
	const ports = $derived(overview.data?.ports ?? []);
	const wan = $derived(live.snapshot?.wan);
	const internet = $derived(live.snapshot?.internet);
	const editingNetwork = $derived(
		editing === 'new' ? 'new' : (networks.find((n) => n.id === editing) ?? null)
	);

	function summary(network: Network) {
		const parts = [network.address ?? 'No address'];
		if (!network.internet) parts.push('no internet');
		if (!network.trusted) parts.push('kept apart');
		return parts.join(' · ');
	}

	function joins(network: Network) {
		const portLabels = ports.filter((p) => network.ports.includes(p.name)).map((p) => p.label);
		const parts = [...network.wifi, ...portLabels];
		return parts.length ? parts.join(', ') : 'Nothing connects to it yet';
	}
</script>

<div class="flex items-end justify-between gap-6">
	<div>
		<h1 class="font-display text-[42px] leading-none font-medium">Networks</h1>
		<p class="mt-2 text-muted">Keep groups of devices apart, and decide what each one can reach.</p>
	</div>
	<Button variant="primary" onclick={() => (editing = 'new')}><Plus size={17} /> New network</Button
	>
</div>

<Section title="Your networks">
	{#each networks as network (network.id)}
		<button
			type="button"
			onclick={() => (editing = network.id)}
			class="flex cursor-pointer items-center gap-4 bg-surface px-6 py-4 text-left transition-colors hover:bg-raised"
		>
			<span class="size-3 shrink-0 rounded-full" style:background={networkColors[network.color]}
			></span>
			<span class="flex min-w-0 flex-1 flex-col">
				<span class="truncate text-[17px] font-medium">{network.name}</span>
				<span class="truncate text-[13px] text-muted">{summary(network)}</span>
			</span>
			<span class="hidden max-w-[45%] truncate text-right text-[13px] text-muted sm:block">
				{joins(network)}
			</span>
		</button>
	{:else}
		<Block description={overview.data ? 'No networks yet.' : 'Loading networks…'} />
	{/each}
</Section>

{#if ports.length}
	<Section title="Ports">
		<div class="grid grid-cols-2 gap-[3px] md:grid-cols-4">
			{#each ports as port (port.name)}
				<PortTile {port} {networks} onselect={(p: Port) => (selectedPort = p.name)} />
			{/each}
		</div>
	</Section>
{/if}

<Section title="Internet">
	<Block>
		<Facts
			facts={[
				{
					label: 'Status',
					value: !wan
						? null
						: internet?.online
							? `Online for ${duration(wan.uptime)}`
							: wan.up
								? 'Connected, no internet'
								: 'Disconnected',
					tone: internet?.online ? 'good' : wan ? 'bad' : undefined
				},
				{ label: 'Connection type', value: wan?.proto?.toUpperCase() },
				{ label: 'Address', value: wan?.ipv4, copy: true },
				{ label: 'Gateway', value: wan?.gateway, copy: true },
				{ label: 'IPv6 prefix', value: wan?.prefix6, copy: true },
				{ label: 'DNS servers', value: wan?.dns.join(', ') },
				{
					label: 'Latency',
					value: internet?.latencyMs != null ? `${internet.latencyMs} ms` : null
				}
			]}
		/>
	</Block>
</Section>

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
