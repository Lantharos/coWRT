<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import { applyChange } from '#lib/api/apply.js';
	import type { Applied, Network, VpnProvider } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import { networkColors } from '#lib/components/networks/colors.js';
	import Button from '#lib/components/ui/Button.svelte';
	import Setting from '#lib/components/ui/Setting.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import Toggle from '#lib/components/ui/Toggle.svelte';

	type Props = {
		provider: VpnProvider | null;
		networks: Network[];
		open: boolean;
		onclose: () => void;
		onsaved: () => void;
	};
	let { provider, networks, open, onclose, onsaved }: Props = $props();

	let enabled = $state(false);
	let name = $state('');
	let config = $state('');
	let chosen = $state<string[]>([]);
	let busy = $state(false);

	$effect.pre(() => {
		if (!open) return;
		untrack(() => {
			if (!provider) return;
			enabled = provider.enabled;
			name = provider.configured ? provider.name : '';
			chosen = [...provider.networks];
			config = '';
		});
	});

	const status = $derived.by(() => {
		if (!provider?.enabled) return null;
		if (!provider.status.handshake)
			return {
				tone: 'text-danger',
				text: 'Not connected. These networks stay offline until it is.'
			};
		return { tone: 'text-accent', text: `Connected to ${provider.endpoint}` };
	});

	async function save() {
		busy = true;
		try {
			const confirmed = await applyChange(
				() =>
					api.put<Applied>('/vpn/client', {
						enabled,
						name: name.trim() || 'VPN provider',
						config: config.trim() || null,
						networks: chosen
					}),
				enabled ? 'Saved' : 'VPN provider switched off'
			);
			if (confirmed) onclose();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
			onsaved();
		}
	}
</script>

<Sheet {open} title="VPN provider" {onclose}>
	<div class="flex flex-col gap-6">
		<div class="flex flex-col divide-y divide-line">
			<Setting
				title="Use a VPN provider"
				hint="Whole networks go through Mullvad, Proton or similar"
			>
				<Toggle bind:checked={enabled} label="Use a VPN provider" />
			</Setting>
			{#if enabled}
				{#each networks as network (network.id)}
					<Setting title={network.name}>
						<span class="flex items-center gap-3">
							<span class="size-2.5 rounded-full" style:background={networkColors[network.color]}
							></span>
							<Toggle
								checked={chosen.includes(network.id)}
								label="Send {network.name} through the VPN"
								onchange={(on) =>
									(chosen = on ? [...chosen, network.id] : chosen.filter((n) => n !== network.id))}
							/>
						</span>
					</Setting>
				{/each}
			{/if}
		</div>

		{#if enabled}
			{#if status}<p class="text-[13px] {status.tone}">{status.text}</p>{/if}
			<TextField bind:value={name} label="Provider" placeholder="Mullvad" />
			<label class="flex flex-col gap-1.5">
				<span class="text-[13px] text-muted">
					{provider?.configured ? 'Replace the WireGuard config' : 'WireGuard config'}
				</span>
				<textarea
					bind:value={config}
					rows="6"
					spellcheck="false"
					placeholder="Paste the file your provider gives you"
					class="w-full resize-none rounded-control bg-raised px-3.5 py-3 font-mono text-[13px] text-fg outline-none placeholder:font-sans placeholder:text-faint focus:bg-overlay"
				></textarea>
			</label>
		{/if}
	</div>

	{#snippet footer()}
		<Button onclick={onclose}>Cancel</Button>
		<Button variant="primary" onclick={save} {busy}>Save</Button>
	{/snippet}
</Sheet>
