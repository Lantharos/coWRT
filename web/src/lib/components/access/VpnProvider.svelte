<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import { applyChange } from '#lib/api/apply.js';
	import type { Applied, Network, VpnProvider } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import { networkColors } from '#lib/components/networks/colors.js';
	import Block from '#lib/components/ui/Block.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import Toggle from '#lib/components/ui/Toggle.svelte';

	type Props = { provider: VpnProvider; networks: Network[]; onsaved: () => void };
	let { provider, networks, onsaved }: Props = $props();

	let enabled = $state(false);
	let name = $state('');
	let config = $state('');
	let chosen = $state<string[]>([]);
	let busy = $state(false);

	const key = $derived(`${provider.enabled}|${provider.name}|${provider.networks.join()}`);

	$effect.pre(() => {
		if (!key) return;
		untrack(() => {
			enabled = provider.enabled;
			name = provider.configured ? provider.name : '';
			chosen = [...provider.networks];
			config = '';
		});
	});

	const changed = $derived(
		enabled !== provider.enabled ||
			config.trim() !== '' ||
			name !== (provider.configured ? provider.name : '') ||
			chosen.slice().sort().join() !== provider.networks.slice().sort().join()
	);

	const status = $derived.by(() => {
		if (!provider.enabled) return null;
		const handshake = provider.status.handshake;
		if (!handshake)
			return {
				tone: 'text-danger',
				text: 'Not connected yet. Traffic on these networks is held back until it is.'
			};
		return { tone: 'text-accent', text: `Connected to ${provider.endpoint}` };
	});

	function toggleNetwork(id: string, on: boolean) {
		chosen = on ? [...chosen, id] : chosen.filter((n) => n !== id);
	}

	async function save() {
		busy = true;
		try {
			await applyChange(
				() =>
					api.put<Applied>('/vpn/client', {
						enabled,
						name: name.trim() || 'VPN provider',
						config: config.trim() || null,
						networks: chosen
					}),
				enabled ? 'Those networks now go through your VPN' : 'VPN provider switched off'
			);
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
			onsaved();
		}
	}
</script>

<Block
	title="Use a VPN provider"
	description="Send whole networks through Mullvad, Proton VPN or any WireGuard provider. If the VPN drops, those devices stay offline instead of leaking."
>
	{#snippet aside()}
		<Toggle bind:checked={enabled} label="Use a VPN provider" />
	{/snippet}
	{#if enabled}
		<div class="flex flex-col gap-5">
			{#if status}<p class="text-[13px] {status.tone}">{status.text}</p>{/if}
			<TextField bind:value={name} label="Provider" placeholder="Mullvad" />
			<label class="flex flex-col gap-1.5">
				<span class="text-[13px] text-muted">
					{provider.configured ? 'Replace the WireGuard config' : 'WireGuard config'}
				</span>
				<textarea
					bind:value={config}
					rows="6"
					spellcheck="false"
					placeholder="[Interface]&#10;PrivateKey = …&#10;Address = …&#10;&#10;[Peer]&#10;…"
					class="w-full resize-none rounded-control bg-raised px-3.5 py-3 font-mono text-[13px] text-fg outline-none placeholder:text-faint focus:bg-overlay"
				></textarea>
			</label>
			<div class="flex flex-col gap-3">
				<span class="text-[13px] text-muted">Networks that use it</span>
				{#each networks as network (network.id)}
					<div class="flex items-center justify-between gap-4">
						<span class="flex items-center gap-2.5 text-[15px]">
							<span class="size-2.5 rounded-full" style:background={networkColors[network.color]}
							></span>
							{network.name}
						</span>
						<Toggle
							checked={chosen.includes(network.id)}
							label="Route {network.name} through the VPN"
							onchange={(on) => toggleNetwork(network.id, on)}
						/>
					</div>
				{/each}
			</div>
		</div>
	{/if}
	{#if changed}
		<Button variant="primary" class="mt-4 w-full" onclick={save} {busy}>Save</Button>
	{/if}
</Block>
