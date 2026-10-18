<script lang="ts">
	import { Download, Trash2 } from '@lucide/svelte';
	import { renderSVG } from 'uqr';
	import { api } from '#lib/api/client.js';
	import type { VpnPeer } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';

	type Props = { peer: VpnPeer | null; onclose: () => void; onsaved: () => void };
	let { peer, onclose, onsaved }: Props = $props();

	let everything = $state(true);
	let config = $state<string | null>(null);
	let busy = $state(false);

	$effect(() => {
		if (!peer) return;
		const id = peer.id;
		const query = everything;
		config = null;
		fetch(`/api/vpn/server/peers/${id}/config?everything=${query}`)
			.then(async (response) => {
				if (!response.ok) throw new Error((await response.json()).error);
				config = await response.text();
			})
			.catch((err) => toasts.show(err.message, 'error'));
	});

	const qr = $derived(config ? renderSVG(config, { border: 1, pixelSize: 6 }) : null);

	function download() {
		if (!config || !peer) return;
		const link = document.createElement('a');
		link.href = URL.createObjectURL(new Blob([config], { type: 'text/plain' }));
		link.download = `${peer.name.replace(/[^\w-]+/g, '-')}.conf`;
		link.click();
		URL.revokeObjectURL(link.href);
	}

	async function remove() {
		if (!peer) return;
		busy = true;
		try {
			await api.delete(`/vpn/server/peers/${peer.id}`);
			toasts.show(`${peer.name} can't connect anymore`);
			onsaved();
			onclose();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<Sheet open={peer !== null} title={peer?.name ?? ''} {onclose}>
	<div class="flex flex-col gap-6">
		<p class="text-[15px] text-muted">
			Scan this with the WireGuard app on the device, or download the file for a computer.
		</p>
		<div
			class="mx-auto grid aspect-square w-full max-w-[280px] place-items-center rounded-[20px] bg-white p-3"
		>
			{#if qr}
				<div class="w-full [&>svg]:h-auto [&>svg]:w-full">{@html qr}</div>
			{:else}
				<span class="size-6 animate-spin rounded-full border-2 border-black/30 border-t-transparent"
				></span>
			{/if}
		</div>
		<div class="flex items-center justify-between gap-6">
			<div>
				<p class="text-[15px]">Send everything through home</p>
				<p class="text-[13px] text-muted">
					Safer on public Wi-Fi. Off means only your home devices go through the VPN.
				</p>
			</div>
			<Toggle bind:checked={everything} label="Send everything through home" />
		</div>
		<Button onclick={download} disabled={!config}><Download size={16} /> Download file</Button>
	</div>

	{#snippet footer()}
		<Button variant="ghost" onclick={remove} {busy} class="mr-auto">
			<Trash2 size={16} /> Remove device
		</Button>
		<Button onclick={onclose}>Done</Button>
	{/snippet}
</Sheet>
