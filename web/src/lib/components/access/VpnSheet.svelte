<script lang="ts">
	import { Plus } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import type { VpnPeer, VpnServer } from '#lib/api/types.js';
	import { ago } from '#lib/format.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Row from '#lib/components/ui/Row.svelte';
	import Select from '#lib/components/ui/controls/Select.svelte';
	import Setting from '#lib/components/ui/Setting.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';
	import PeerSheet from './PeerSheet.svelte';

	type Props = {
		server: VpnServer | null;
		open: boolean;
		onclose: () => void;
		onsaved: () => void;
	};
	let { server, open, onclose, onsaved }: Props = $props();

	let busy = $state(false);
	let adding = $state('');
	let selected = $state<string | null>(null);

	async function save(changes: Partial<VpnServer>, message: string) {
		if (!server) return;
		busy = true;
		if (changes.enabled && !server.enabled) toasts.show('Setting up WireGuard');
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

	async function add(event: SubmitEvent) {
		event.preventDefault();
		if (!adding.trim()) return;
		busy = true;
		try {
			await api.post('/vpn/server/peers', { name: adding.trim() });
			adding = '';
			onsaved();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}

	function seen(peer: VpnPeer) {
		if (!peer.status.handshake) return 'Never connected';
		if (Date.now() / 1000 - peer.status.handshake < 180) return 'Connected';
		return ago(peer.status.handshake);
	}
</script>

<Sheet {open} title="VPN" {onclose}>
	{#if server}
		<div class="flex flex-col gap-6">
			<div class="flex flex-col divide-y divide-line">
				<Setting
					title="Connect from anywhere"
					hint={server.enabled && server.endpoint
						? `${server.endpoint}, port ${server.port}`
						: 'Your phone and laptop reach home over WireGuard'}
				>
					<Toggle
						checked={server.enabled}
						label="VPN"
						disabled={busy}
						onchange={(on) => save({ enabled: on }, on ? 'Your VPN is ready' : 'VPN switched off')}
					/>
				</Setting>
				{#if server.enabled}
					<Setting title="Devices on it can reach">
						<div class="w-44">
							<Select
								value={server.access}
								onchange={(access) => save({ access }, 'Saved')}
								options={[
									{ value: 'everything', label: 'Everything' },
									{ value: 'internet', label: 'Only the internet' }
								]}
							/>
						</div>
					</Setting>
				{/if}
			</div>

			{#if server.enabled}
				<div class="-mx-3 flex flex-col">
					{#each server.peers as peer (peer.id)}
						<Row
							title={peer.name}
							value={seen(peer)}
							tone={peer.status.handshake ? 'good' : 'muted'}
							onclick={() => (selected = peer.id)}
						/>
					{/each}
				</div>
				<form class="flex items-end gap-2" onsubmit={add}>
					<TextField bind:value={adding} placeholder="Add a device, like My phone" class="flex-1" />
					<Button type="submit" class="h-11" {busy} disabled={!adding.trim()}>
						<Plus size={16} /> Add
					</Button>
				</form>
			{/if}
		</div>
	{/if}
</Sheet>

<PeerSheet
	peer={server?.peers.find((p) => p.id === selected) ?? null}
	onclose={() => (selected = null)}
	{onsaved}
/>
