<script lang="ts">
	import { Plus } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { Device, Forward, SharingOverview } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Ddns from '#lib/components/access/Ddns.svelte';
	import ForwardSheet from '#lib/components/access/ForwardSheet.svelte';
	import Reachability from '#lib/components/access/Reachability.svelte';
	import Upnp from '#lib/components/access/Upnp.svelte';
	import Block from '#lib/components/ui/Block.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Section from '#lib/components/ui/Section.svelte';
	import Toggle from '#lib/components/ui/Toggle.svelte';

	const sharing = poll(() => api.get<SharingOverview>('/sharing'), 30000);
	const devices = poll(() => api.get<Device[]>('/devices'), 30000);

	let editing = $state<string | 'new' | null>(null);

	const forwards = $derived(sharing.data?.forwards ?? []);
	const deviceName = (mac: string) => devices.data?.find((d) => d.mac === mac)?.name ?? mac;
	const editingForward = $derived(
		editing === 'new' ? 'new' : (forwards.find((f) => f.id === editing) ?? null)
	);

	async function setEnabled(forward: Forward, enabled: boolean) {
		try {
			await api.put(`/forwards/${forward.id}`, { ...forward, enabled });
			toasts.show(enabled ? `${forward.name} is open` : `${forward.name} is closed`);
		} catch (err) {
			toasts.error(err);
		}
		sharing.refresh();
	}

	const protocolLabel = { tcp: 'TCP', udp: 'UDP', both: 'TCP and UDP' };
</script>

<div class="flex items-end justify-between gap-6">
	<div>
		<h1 class="font-display text-[42px] leading-none font-medium">Access</h1>
		<p class="mt-2 text-muted">Let things on the internet reach home, safely.</p>
	</div>
	<Button variant="primary" onclick={() => (editing = 'new')}>
		<Plus size={17} /> Port forward
	</Button>
</div>

{#if sharing.data}
	<Section title="From outside">
		<Reachability reach={sharing.data.reach} />
		<Ddns ddns={sharing.data.ddns} onsaved={() => sharing.refresh()} />
	</Section>

	<Section title="Port forwards">
		{#each forwards as forward (forward.id)}
			<div class="flex items-center gap-4 bg-surface pr-6 transition-colors hover:bg-raised">
				<button
					type="button"
					onclick={() => (editing = forward.id)}
					class="flex min-w-0 flex-1 cursor-pointer flex-col py-4 pl-6 text-left"
				>
					<span class="truncate text-[17px] font-medium {forward.enabled ? '' : 'text-muted'}">
						{forward.name}
					</span>
					<span class="truncate text-[13px] text-muted">
						{protocolLabel[forward.protocol]}
						{forward.external} → {deviceName(forward.mac)}{forward.internal
							? `:${forward.internal}`
							: ''}
					</span>
				</button>
				<Toggle
					checked={forward.enabled}
					label="Open {forward.name}"
					onchange={(on) => setEnabled(forward, on)}
				/>
			</div>
		{:else}
			<Block description="Nothing is forwarded. Add one when a game or server needs it." />
		{/each}
		<Upnp upnp={sharing.data.upnp} onsaved={() => sharing.refresh()} />
	</Section>
{:else}
	<Block description="Checking how the internet sees you…" />
{/if}

<ForwardSheet
	forward={editingForward}
	devices={devices.data ?? []}
	onclose={() => (editing = null)}
	onsaved={() => sharing.refresh()}
/>
