<script lang="ts">
	import { untrack } from 'svelte';
	import { Check } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { applyChange } from '#lib/api/apply.js';
	import type { Applied, Network, Port } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';
	import { networkColors } from './colors';

	type Props = {
		port: Port | null;
		networks: Network[];
		onclose: () => void;
		onsaved: () => void;
	};
	let { port, networks, onclose, onsaved }: Props = $props();

	let network = $state<string | null>(null);
	let tagged = $state<string[]>([]);
	let busy = $state(false);

	const key = $derived(port?.name);

	$effect.pre(() => {
		if (!key) return;
		untrack(() => {
			network = port?.network ?? null;
			tagged = [...(port?.tagged ?? [])];
		});
	});

	const others = $derived(networks.filter((n) => n.id !== network));

	function toggleTagged(id: string, on: boolean) {
		tagged = on ? [...tagged, id] : tagged.filter((t) => t !== id);
	}

	async function save() {
		if (!port) return;
		busy = true;
		try {
			const body = { network, tagged: tagged.filter((id) => id !== network) };
			await applyChange(
				() => api.put<Applied>(`/ports/${port.name}`, body),
				`${port.label} updated`
			);
			onsaved();
			onclose();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<Sheet open={port !== null} title={port?.label ?? ''} {onclose}>
	<div class="flex flex-col gap-8">
		<div class="flex flex-col gap-3">
			<p class="text-sm text-muted">Anything plugged in here joins</p>
			<div class="flex flex-col gap-1">
				{#each networks as option (option.id)}
					{@render choice(option.id, option.name, option.address, networkColors[option.color])}
				{/each}
				{@render choice(null, 'Nothing', 'The port stays switched off', null)}
			</div>
		</div>

		{#if others.length}
			<div class="flex flex-col gap-3">
				<div>
					<h3 class="text-[15px]">Also carry</h3>
					<p class="text-[13px] text-muted">Tagged, for switches and access points</p>
				</div>
				{#each others as option (option.id)}
					<div class="flex items-center justify-between gap-4">
						<span class="flex items-center gap-2.5">
							<span class="size-2.5 rounded-full" style:background={networkColors[option.color]}
							></span>
							{option.name}
							<span class="text-[13px] text-faint">VLAN {option.vlan}</span>
						</span>
						<Toggle
							checked={tagged.includes(option.id)}
							label="Carry {option.name}"
							onchange={(on) => toggleTagged(option.id, on)}
						/>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	{#snippet footer()}
		<Button onclick={onclose}>Cancel</Button>
		<Button variant="primary" onclick={save} {busy}>Save</Button>
	{/snippet}
</Sheet>

{#snippet choice(id: string | null, name: string, detail: string | null, color: string | null)}
	<button
		type="button"
		onclick={() => (network = id)}
		class="flex cursor-pointer items-center gap-3 rounded-control px-3.5 py-3 text-left transition-colors {network ===
		id
			? 'bg-raised'
			: 'hover:bg-raised/60'}"
	>
		<span
			class="size-3 shrink-0 rounded-full {color ? '' : 'border border-faint'}"
			style:background={color}
		></span>
		<span class="flex min-w-0 flex-1 flex-col">
			<span class="text-[15px]">{name}</span>
			{#if detail}<span class="text-[13px] text-muted">{detail}</span>{/if}
		</span>
		{#if network === id}<Check size={17} class="text-accent" />{/if}
	</button>
{/snippet}
