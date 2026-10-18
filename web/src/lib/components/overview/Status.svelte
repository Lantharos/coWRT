<script lang="ts">
	import { PlugZap, Smartphone, Unplug } from '@lucide/svelte';
	import type { Snapshot } from '#lib/api/types.js';
	import { duration } from '#lib/format.js';

	let { snapshot }: { snapshot: Snapshot | null } = $props();

	const backup = $derived(
		snapshot?.uplink?.kind === 'phone' ? 'your phone' : (snapshot?.uplink?.name ?? '')
	);

	const state = $derived.by(() => {
		if (!snapshot || !snapshot.internet.checked) return 'checking';
		if (snapshot.internet.online && snapshot.uplink && snapshot.uplink.kind !== 'cable')
			return 'backup';
		if (snapshot.internet.online) return 'online';
		return snapshot.wan.up ? 'unreachable' : 'down';
	});

	const detail = $derived.by(() => {
		if (!snapshot) return 'Talking to your router…';
		const { wan, internet } = snapshot;
		switch (state) {
			case 'online':
				return [
					`Up for ${duration(wan.uptime)}`,
					internet.latencyMs !== null && `${internet.latencyMs} ms to the internet`,
					internet.online6 ? 'IPv4 and IPv6' : 'IPv4 only'
				]
					.filter(Boolean)
					.join(' · ');
			case 'backup':
				return `Your main line is down, so the router is using ${backup}${internet.latencyMs !== null ? ` · ${internet.latencyMs} ms` : ''}`;
			case 'unreachable':
				return 'Your router has an address, but nothing on the internet answers. This is usually your provider.';
			case 'down':
				return 'The internet port has no connection. Check the cable to your modem.';
			default:
				return 'Checking your connection…';
		}
	});
</script>

<div class="flex flex-col gap-3">
	<h1
		class="font-display text-[clamp(2rem,5vw,3.25rem)] leading-[1.05] font-medium tracking-[-0.01em]"
	>
		{#if state === 'online'}
			You're <span class="inline-flex items-center gap-2 text-accent"
				><PlugZap class="size-[0.85em]" strokeWidth={2.2} />connected</span
			> to the internet
		{:else if state === 'backup'}
			You're on <span class="inline-flex items-center gap-2 text-warn"
				><Smartphone class="size-[0.85em]" strokeWidth={2.2} />{backup}</span
			>
		{:else if state === 'checking'}
			<span class="text-muted">Checking your connection</span>
		{:else}
			You're <span class="inline-flex items-center gap-2 text-danger"
				><Unplug class="size-[0.85em]" strokeWidth={2.2} />offline</span
			>
		{/if}
	</h1>
	<p class="max-w-[60ch] text-[17px] text-muted">{detail}</p>
</div>
