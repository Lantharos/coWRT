<script lang="ts">
	import { ArrowDown, ArrowUp, ChevronRight } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { live } from '#lib/api/live.svelte.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { Device, WifiOverview } from '#lib/api/types.js';
	import { bandLabel, bits, bytes } from '#lib/format.js';
	import GridFloor from '#lib/components/charts/GridFloor.svelte';
	import DeviceRow from '#lib/components/devices/DeviceRow.svelte';
	import DeviceSheet from '#lib/components/devices/DeviceSheet.svelte';
	import Stat from '#lib/components/overview/Stat.svelte';
	import Status from '#lib/components/overview/Status.svelte';
	import Block from '#lib/components/ui/Block.svelte';
	import Facts from '#lib/components/ui/Facts.svelte';
	import Section from '#lib/components/ui/Section.svelte';

	const devices = poll(() => api.get<Device[]>('/devices'), 5000);
	const wifi = poll(() => api.get<WifiOverview>('/wifi'), 15000);

	let selected = $state<string | null>(null);

	const snapshot = $derived(live.snapshot);
	const online = $derived(devices.data?.filter((device) => device.online) ?? []);
	const used = $derived(bytes(snapshot?.usageToday ?? 0));
	const down = $derived(bits(snapshot?.throughput.rx ?? 0));
	const up = $derived(bits(snapshot?.throughput.tx ?? 0));
	const levels = $derived(
		live.samples.map((sample) => sample.throughput.rx + sample.throughput.tx)
	);
</script>

<div class="flex flex-col gap-12">
	<div class="flex flex-col gap-12 pt-6">
		<Status {snapshot} />
		<div class="flex flex-wrap gap-x-14 gap-y-8">
			<Stat value={devices.data ? String(online.length) : '–'} label="devices online" />
			<Stat value={used.value} unit={used.unit} label="used today" />
			<Stat value={down.value} unit={down.unit} label="download right now">
				{#snippet lead()}<ArrowDown class="size-7 text-accent" />{/snippet}
			</Stat>
			<Stat value={up.value} unit={up.unit} label="upload right now">
				{#snippet lead()}<ArrowUp class="size-7 text-muted" />{/snippet}
			</Stat>
		</div>
	</div>
	<div
		class="pointer-events-none relative left-1/2 -mt-6 h-[180px] w-screen -translate-x-1/2 sm:h-[260px]"
	>
		<GridFloor {levels} />
	</div>
</div>

<div class="grid gap-12 lg:grid-cols-[1.35fr_1fr]">
	<Section title="Online now">
		{#snippet actions()}
			<a href="/devices" class="flex items-center gap-1 text-sm text-muted hover:text-fg">
				All devices <ChevronRight size={16} />
			</a>
		{/snippet}
		{#each online.slice(0, 6) as device (device.mac)}
			<DeviceRow {device} onselect={(d) => (selected = d.mac)} />
		{:else}
			<Block
				description={devices.data ? 'Nothing is connected right now.' : 'Looking for devices…'}
			/>
		{/each}
	</Section>

	<div class="flex flex-col gap-12">
		<Section title="Wi-Fi">
			{#snippet actions()}
				<a href="/wifi" class="flex items-center gap-1 text-sm text-muted hover:text-fg">
					Manage <ChevronRight size={16} />
				</a>
			{/snippet}
			{#each wifi.data?.networks ?? [] as network (network.id)}
				<a
					href="/wifi"
					class="flex items-center justify-between gap-4 bg-surface px-6 py-4 transition-colors hover:bg-raised"
				>
					<span class="flex min-w-0 flex-col">
						<span class="truncate font-medium">{network.ssid}</span>
						<span class="text-[13px] text-muted">
							{network.enabled
								? network.bands.map((band) => bandLabel[band]).join(' & ')
								: 'Turned off'}
						</span>
					</span>
					<span class="shrink-0 text-sm text-muted">
						{network.clients}
						{network.clients === 1 ? 'device' : 'devices'}
					</span>
				</a>
			{:else}
				<Block description="Loading networks…" />
			{/each}
		</Section>

		<Section title="Connection">
			<Block>
				<Facts
					facts={[
						{ label: 'Internet address', value: snapshot?.wan.ipv4?.split('/')[0], copy: true },
						{ label: 'IPv6 prefix', value: snapshot?.wan.prefix6, copy: true },
						{ label: 'Gateway', value: snapshot?.wan.gateway },
						{ label: 'DNS', value: snapshot?.wan.dns.join(', ') },
						{ label: 'Type', value: snapshot?.wan.proto?.toUpperCase() }
					]}
				/>
			</Block>
		</Section>
	</div>
</div>

<DeviceSheet
	device={devices.data?.find((device) => device.mac === selected) ?? null}
	onclose={() => (selected = null)}
	onchange={() => devices.refresh()}
/>
