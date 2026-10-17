<script lang="ts">
	import { untrack } from 'svelte';
	import { Trash2 } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { applyChange } from '#lib/api/apply.js';
	import type { Applied, Network, NetworkColor, NetworkInput } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/Button.svelte';
	import Select from '#lib/components/ui/Select.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import Toggle from '#lib/components/ui/Toggle.svelte';
	import { networkColors } from './colors';

	type Props = {
		network: Network | 'new' | null;
		networks: Network[];
		onclose: () => void;
		onsaved: () => void;
	};
	let { network, networks, onclose, onsaved }: Props = $props();

	const leaseTimes = [
		{ value: '1h', label: '1 hour' },
		{ value: '2h', label: '2 hours' },
		{ value: '12h', label: '12 hours' },
		{ value: '1d', label: '1 day' },
		{ value: '7d', label: '1 week' }
	];

	let form = $state<NetworkInput>(blank());
	let start = $state('100');
	let limit = $state('150');
	let vlan = $state('');
	let busy = $state<'save' | 'delete' | null>(null);
	let confirmDelete = $state(false);

	function blank(): NetworkInput {
		const used = new Set(networks.map((n) => n.vlan));
		let next = 10;
		while (used.has(next)) next += 10;
		const taken = new Set(networks.map((n) => n.color));
		const color =
			(Object.keys(networkColors) as NetworkColor[]).find((c) => !taken.has(c)) ?? 'blue';
		return {
			name: '',
			color,
			vlan: next,
			address: next <= 255 ? `10.0.${next}.1/24` : '',
			dhcp: { enabled: true, start: 100, limit: 150, leaseTime: '12h' },
			trusted: false,
			internet: true
		};
	}

	const key = $derived(network === 'new' ? 'new' : network?.id);

	$effect.pre(() => {
		if (!key) return;
		untrack(() => {
			if (network === null) return;
			confirmDelete = false;
			form =
				network === 'new'
					? blank()
					: {
							name: network.name,
							color: network.color,
							vlan: network.vlan,
							address: network.address ?? '',
							dhcp: network.dhcp ?? { enabled: false, start: 100, limit: 150, leaseTime: '12h' },
							trusted: network.trusted,
							internet: network.internet
						};
			start = String(form.dhcp.start);
			limit = String(form.dhcp.limit);
			vlan = String(form.vlan);
		});
	});

	const primary = $derived(network !== 'new' && network?.primary === true);
	const pool = $derived.by(() => {
		const base = form.address.split('/')[0].split('.').slice(0, 3).join('.');
		const first = Number(start);
		const count = Number(limit);
		if (!base || !first || !count) return null;
		return `${base}.${first} to ${base}.${first + count - 1}`;
	});

	async function save() {
		if (network === null) return;
		busy = 'save';
		const body: NetworkInput = {
			...form,
			vlan: Number(vlan),
			dhcp: { ...form.dhcp, start: Number(start), limit: Number(limit) }
		};
		try {
			const saved = await applyChange(
				() =>
					network === 'new'
						? api.post<Applied>('/networks', body)
						: api.put<Applied>(`/networks/${network.id}`, body),
				network === 'new' ? `${body.name} is ready` : `${body.name} saved`
			);
			onsaved();
			if (saved) onclose();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}

	async function remove() {
		if (!network || network === 'new') return;
		if (!confirmDelete) {
			confirmDelete = true;
			return;
		}
		busy = 'delete';
		try {
			await applyChange(
				() => api.delete<Applied>(`/networks/${network.id}`),
				`${network.name} removed`
			);
			onsaved();
			onclose();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}
</script>

<Sheet
	open={network !== null}
	title={network === 'new' ? 'New network' : (network?.name ?? '')}
	{onclose}
>
	<div class="flex flex-col gap-6">
		<TextField
			bind:value={form.name}
			label="Name"
			placeholder="Guests, Smart home, Office…"
			maxlength={32}
		/>

		<div class="flex flex-col gap-1.5">
			<span class="text-[13px] text-muted">Color</span>
			<div class="flex gap-2">
				{#each Object.entries(networkColors) as [name, hex] (name)}
					<button
						type="button"
						aria-label={name}
						aria-pressed={form.color === name}
						onclick={() => (form.color = name as NetworkColor)}
						class="grid size-9 cursor-pointer place-items-center rounded-full transition-transform hover:scale-110 {form.color ===
						name
							? 'ring-2 ring-fg ring-offset-2 ring-offset-surface'
							: ''}"
						style:background={hex}
					></button>
				{/each}
			</div>
		</div>

		<div class="flex flex-col gap-5 pt-1">
			{@render option(
				'Internet access',
				'Devices on this network can reach the internet.',
				form.internet,
				(on) => (form.internet = on)
			)}
			{@render option(
				'Reach other networks',
				primary
					? 'Your main network can always reach everything, including this router.'
					: 'Devices here can reach your other networks and manage this router. Leave off for guests and smart home gear.',
				form.trusted || primary,
				(on) => (form.trusted = on),
				primary
			)}
		</div>

		<TextField
			bind:value={form.address}
			label="Router address"
			hint="The router's address on this network, with its size. 10.0.20.1/24 gives 254 devices."
			placeholder="10.0.20.1/24"
		/>

		<div class="flex flex-col gap-4 rounded-[18px] bg-raised/60 p-4">
			{@render option(
				'Hand out addresses',
				pool && form.dhcp.enabled ? `From ${pool}` : 'Devices need to be set up by hand.',
				form.dhcp.enabled,
				(on) => (form.dhcp.enabled = on)
			)}
			{#if form.dhcp.enabled}
				<div class="grid grid-cols-2 gap-3">
					<TextField bind:value={start} label="Starting at" inputmode="numeric" />
					<TextField bind:value={limit} label="How many" inputmode="numeric" />
				</div>
				<Select
					bind:value={form.dhcp.leaseTime}
					options={leaseTimes}
					label="Devices keep their address for"
				/>
			{/if}
		</div>

		<TextField
			bind:value={vlan}
			label="VLAN number"
			hint="Only matters if you connect other switches or access points that carry several networks."
			inputmode="numeric"
		/>
	</div>

	{#snippet footer()}
		{#if network !== 'new' && !primary}
			<Button
				variant={confirmDelete ? 'danger' : 'ghost'}
				onclick={remove}
				busy={busy === 'delete'}
				class="mr-auto"
			>
				<Trash2 size={16} />
				{confirmDelete ? 'Really remove?' : 'Remove'}
			</Button>
		{/if}
		<Button onclick={onclose}>Cancel</Button>
		<Button variant="primary" onclick={save} busy={busy === 'save'} disabled={!form.name.trim()}>
			{network === 'new' ? 'Create' : 'Save'}
		</Button>
	{/snippet}
</Sheet>

{#snippet option(
	title: string,
	description: string,
	checked: boolean,
	onchange: (on: boolean) => void,
	disabled = false
)}
	<div class="flex items-center justify-between gap-6">
		<div>
			<p class="text-[15px]">{title}</p>
			<p class="text-[13px] text-muted">{description}</p>
		</div>
		<Toggle {checked} label={title} {onchange} {disabled} />
	</div>
{/snippet}
