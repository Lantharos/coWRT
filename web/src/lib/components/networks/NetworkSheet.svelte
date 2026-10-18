<script lang="ts">
	import { untrack } from 'svelte';
	import { Trash2 } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { applyChange } from '#lib/api/apply.js';
	import type { Applied, Network, Color, NetworkInput } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import ColorPicker from '#lib/components/ui/controls/ColorPicker.svelte';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Select from '#lib/components/ui/controls/Select.svelte';
	import Setting from '#lib/components/ui/Setting.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';
	import { palette } from '#lib/palette.js';

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
		const color = (Object.keys(palette) as Color[]).find((c) => !taken.has(c)) ?? 'blue';
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

		<ColorPicker bind:value={form.color} />

		<TextField bind:value={form.address} label="Router address" placeholder="10.0.20.1/24" />

		<div class="flex flex-col">
			<Setting title="Internet">
				<Toggle bind:checked={form.internet} label="Internet" />
			</Setting>
			<Setting
				title="Reach other networks"
				hint={primary ? 'Always on for your main network' : 'Leave off for guests and gadgets'}
			>
				<Toggle
					checked={form.trusted || primary}
					label="Reach other networks"
					disabled={primary}
					onchange={(on) => (form.trusted = on)}
				/>
			</Setting>
			<Setting title="Hand out addresses" hint={pool && form.dhcp.enabled ? pool : undefined}>
				<Toggle bind:checked={form.dhcp.enabled} label="Hand out addresses" />
			</Setting>
		</div>

		{#if form.dhcp.enabled}
			<div class="grid grid-cols-2 gap-3">
				<TextField bind:value={start} label="First address" inputmode="numeric" />
				<TextField bind:value={limit} label="How many" inputmode="numeric" />
			</div>
			<Select bind:value={form.dhcp.leaseTime} options={leaseTimes} label="Kept for" />
		{/if}

		<TextField
			bind:value={vlan}
			label="VLAN number"
			hint="For switches and access points that carry several networks"
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
