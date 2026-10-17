<script lang="ts">
	import { Plus } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import type { VpnPeer, VpnServer } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Block from '#lib/components/ui/Block.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Segmented from '#lib/components/ui/Segmented.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import Toggle from '#lib/components/ui/Toggle.svelte';
	import PeerSheet from './PeerSheet.svelte';

	let { server, onsaved }: { server: VpnServer; onsaved: () => void } = $props();

	let busy = $state(false);
	let adding = $state<string | null>(null);
	let selected = $state<string | null>(null);

	async function save(changes: Partial<VpnServer>, message: string) {
		busy = true;
		if (changes.enabled && !server.enabled)
			toasts.show('Setting up WireGuard, this takes a moment');
		try {
			await api.put('/vpn/server', {
				enabled: changes.enabled ?? server.enabled,
				port: server.port,
				access: changes.access ?? server.access
			});
			toasts.show(message);
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
			onsaved();
		}
	}

	async function add() {
		if (!adding?.trim()) return;
		busy = true;
		try {
			await api.post('/vpn/server/peers', { name: adding.trim() });
			adding = null;
			onsaved();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}

	function seen(peer: VpnPeer) {
		if (!peer.status.handshake) return 'Never connected';
		const seconds = Date.now() / 1000 - peer.status.handshake;
		if (seconds < 180) return 'Connected now';
		if (seconds < 3600) return `Seen ${Math.round(seconds / 60)} min ago`;
		if (seconds < 86400) return `Seen ${Math.round(seconds / 3600)} h ago`;
		return `Seen ${Math.round(seconds / 86400)} days ago`;
	}
</script>

<Block
	title="Connect to home"
	description={server.enabled
		? `Your devices reach home at ${server.endpoint ?? 'your public address'}, port ${server.port}.`
		: 'A private WireGuard VPN so your phone and laptop can reach home from anywhere.'}
>
	{#snippet aside()}
		<Toggle
			checked={server.enabled}
			label="VPN server"
			disabled={busy}
			onchange={(on) => save({ enabled: on }, on ? 'Your VPN is ready' : 'VPN switched off')}
		/>
	{/snippet}
	{#if server.enabled}
		<div class="flex flex-col gap-5">
			<Segmented
				value={server.access}
				label="Devices on the VPN can reach"
				options={[
					{ value: 'everything', label: 'Everything at home' },
					{ value: 'internet', label: 'Only the internet' }
				]}
				onchange={(access) => save({ access }, 'Access updated')}
			/>
			<div class="flex flex-col gap-1">
				{#each server.peers as peer (peer.id)}
					<button
						type="button"
						onclick={() => (selected = peer.id)}
						class="-mx-3 flex cursor-pointer items-center justify-between gap-4 rounded-control px-3 py-2.5 text-left hover:bg-raised"
					>
						<span class="text-[15px]">{peer.name}</span>
						<span class="text-[13px] {peer.status.handshake ? 'text-accent' : 'text-muted'}"
							>{seen(peer)}</span
						>
					</button>
				{/each}
			</div>
			{#if adding !== null}
				<form
					class="flex items-end gap-2"
					onsubmit={(event) => {
						event.preventDefault();
						add();
					}}
				>
					<TextField
						bind:value={adding}
						label="Device name"
						placeholder="My phone"
						class="flex-1"
						autofocus
					/>
					<Button variant="primary" type="submit" {busy} class="h-11">Add</Button>
				</form>
			{:else}
				<Button size="sm" variant="ghost" class="self-start" onclick={() => (adding = '')}>
					<Plus size={15} /> Add a device
				</Button>
			{/if}
		</div>
	{/if}
</Block>

<PeerSheet
	peer={server.peers.find((p) => p.id === selected) ?? null}
	onclose={() => (selected = null)}
	{onsaved}
/>
