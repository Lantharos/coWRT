<script lang="ts">
	import { Gauge, Shuffle, Smartphone } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { live } from '#lib/api/live.svelte.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import { speedtests } from '#lib/api/speedtest.svelte.js';
	import type { InternetOverview, Uplink, UplinkStatus } from '#lib/api/types.js';
	import { ago } from '#lib/format.js';
	import List from '#lib/components/ui/List.svelte';
	import Row from '#lib/components/ui/Row.svelte';
	import FailoverSheet from './FailoverSheet.svelte';
	import { mbps } from './grade';
	import InternetSheet from './InternetSheet.svelte';
	import PhoneSheet from './PhoneSheet.svelte';
	import SpeedSheet from './SpeedSheet.svelte';
	import { uplinkIcon, uplinkState } from './uplinks';

	let { speedOpen = $bindable(false) }: { speedOpen?: boolean } = $props();

	const settings = poll(() => api.get<InternetOverview>('/internet'), 60000);
	const status = poll(() => api.get<UplinkStatus>('/internet/uplinks'), 5000);
	speedtests.use();

	let panel = $state<'main' | 'failover' | 'phone' | null>(null);

	const uplinks = $derived(status.data?.uplinks ?? []);
	const phone = $derived(uplinks.find((u) => u.kind === 'phone') ?? null);
	const connectionKind = $derived(
		{ dhcp: 'Automatic', static: 'Fixed address', pppoe: 'PPPoE' }[
			settings.data?.connection.kind ?? 'dhcp'
		]
	);

	function detail(uplink: Uplink, index: number) {
		const wan = live.snapshot?.wan;
		const addresses =
			uplink.inUse && wan ? [wan.ipv4?.split('/')[0], wan.prefix6 ?? wan.ipv6?.split('/')[0]] : [];
		return [index === 0 ? connectionKind : null, uplinkState(uplink), ...addresses]
			.filter(Boolean)
			.join(' · ');
	}

	const failoverValue = $derived.by(() => {
		const mode = status.data?.mode;
		if (!mode || mode === 'auto') return 'Automatic';
		return `Always ${uplinks.find((u) => u.id === mode)?.name.toLowerCase() ?? mode}`;
	});

	const speed = $derived.by(() => {
		const running = speedtests.data?.running;
		const latest = speedtests.latest;
		if (running) return { detail: 'Testing…', value: `${mbps(running.current)} Mbps` };
		if (!latest) return { detail: 'Measure your line', value: null };
		return {
			detail: `Tested ${ago(latest.time)}`,
			value: `${mbps(latest.download)} / ${mbps(latest.upload)} Mbps`
		};
	});

	const open = (uplink: Uplink) => (panel = uplink.kind === 'phone' ? 'phone' : 'main');
	const refresh = () => {
		status.refresh();
		settings.refresh();
	};
</script>

<List label="Internet">
	{#each uplinks as uplink, index (uplink.id)}
		<Row
			title={uplink.name}
			detail={detail(uplink, index)}
			icon={uplinkIcon[uplink.kind]}
			tint={uplink.inUse
				? uplink.online
					? 'var(--color-accent)'
					: 'var(--color-danger)'
				: uplink.online
					? 'var(--color-warn)'
					: undefined}
			dimmed={!uplink.connected}
			graph={uplink.inUse
				? live.samples.map((sample) => sample.throughput.rx + sample.throughput.tx)
				: undefined}
			value={uplink.online && uplink.latencyMs !== null ? `${uplink.latencyMs} ms` : null}
			onclick={() => open(uplink)}
		/>
	{/each}
	{#if uplinks.length > 1}
		<Row
			title="Failover"
			detail="Which connection to use"
			icon={Shuffle}
			value={failoverValue}
			onclick={() => (panel = 'failover')}
		/>
	{:else if status.data}
		<Row
			title="Phone as backup"
			detail="Keeps you online when the main line drops"
			icon={Smartphone}
			value="Off"
			onclick={() => (panel = 'phone')}
		/>
	{/if}
	<Row
		title="Speed"
		icon={Gauge}
		tint={speedtests.latest ? 'var(--color-accent)' : undefined}
		detail={speed.detail}
		graph={speedtests.data?.running?.trace ??
			speedtests.data?.history.map((result) => result.download)}
		value={speed.value}
		onclick={() => (speedOpen = true)}
	/>
</List>

<SpeedSheet open={speedOpen} onclose={() => (speedOpen = false)} />
<InternetSheet
	settings={settings.data}
	open={panel === 'main'}
	onclose={() => (panel = null)}
	onsaved={refresh}
/>
<FailoverSheet
	status={status.data}
	open={panel === 'failover'}
	onclose={() => (panel = null)}
	onsaved={refresh}
/>
<PhoneSheet {phone} open={panel === 'phone'} onclose={() => (panel = null)} onsaved={refresh} />
