<script lang="ts">
	import { api } from '#lib/api/client.js';
	import { live } from '#lib/api/live.svelte.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import { duration } from '#lib/format.js';
	import Block from '#lib/components/ui/Block.svelte';
	import Facts from '#lib/components/ui/Facts.svelte';
	import Section from '#lib/components/ui/Section.svelte';

	type Lan = {
		device: string | null;
		ipv4: string | null;
		ipv6: string[];
		dhcp: { enabled: boolean; start: number; limit: number; leaseTime: string } | null;
	};

	const lan = poll(() => api.get<Lan>('/network/lan'), 30000);

	const wan = $derived(live.snapshot?.wan);
	const internet = $derived(live.snapshot?.internet);

	const pool = $derived.by(() => {
		const dhcp = lan.data?.dhcp;
		const base = lan.data?.ipv4?.split('/')[0].split('.').slice(0, 3).join('.');
		if (!dhcp || !base) return null;
		return `${base}.${dhcp.start} to ${base}.${dhcp.start + dhcp.limit - 1}`;
	});
</script>

<div>
	<h1 class="font-display text-[42px] leading-none font-medium">Network</h1>
	<p class="mt-2 text-muted">How your router talks to the internet and to your home.</p>
</div>

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
				{ label: 'Port', value: wan?.device },
				{ label: 'IPv4 address', value: wan?.ipv4, copy: true },
				{ label: 'Gateway', value: wan?.gateway, copy: true },
				{ label: 'IPv6 address', value: wan?.ipv6, copy: true },
				{ label: 'IPv6 prefix', value: wan?.prefix6, copy: true },
				{ label: 'DNS servers', value: wan?.dns.join(', ') },
				{ label: 'Latency', value: internet?.latencyMs != null ? `${internet.latencyMs} ms` : null }
			]}
		/>
	</Block>
</Section>

<Section title="Home network">
	<Block>
		<Facts
			facts={[
				{ label: 'Router address', value: lan.data?.ipv4, copy: true },
				{ label: 'IPv6', value: lan.data?.ipv6.join(', ') },
				{ label: 'Interface', value: lan.data?.device },
				{
					label: 'DHCP',
					value: lan.data?.dhcp ? (lan.data.dhcp.enabled ? `Hands out ${pool}` : 'Off') : null
				},
				{ label: 'Lease time', value: lan.data?.dhcp?.leaseTime }
			]}
		/>
	</Block>
</Section>
