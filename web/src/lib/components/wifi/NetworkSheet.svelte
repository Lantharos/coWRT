<script lang="ts">
	import { untrack } from 'svelte';
	import { Trash2 } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import type {
		Band,
		Network,
		Radio,
		Security,
		WifiNetwork,
		WifiNetworkInput
	} from '#lib/api/types.js';
	import { bandLabel } from '#lib/format.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/Button.svelte';
	import Segmented from '#lib/components/ui/Segmented.svelte';
	import Select from '#lib/components/ui/Select.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import Toggle from '#lib/components/ui/Toggle.svelte';

	type Props = {
		network: WifiNetwork | 'new' | null;
		radios: Radio[];
		networks: Network[];
		connectedHere: boolean;
		onclose: () => void;
		onsaved: () => void;
	};
	let { network, radios, networks, connectedHere, onclose, onsaved }: Props = $props();

	const networkOptions = $derived(
		networks.map((n) => ({ value: n.id, label: n.name, detail: n.address ?? undefined }))
	);

	const bands = $derived([...new Set(radios.map((radio) => radio.band))] as Band[]);

	const securityOptions: { value: Security; label: string; detail: string }[] = [
		{
			value: 'wpa2-wpa3',
			label: 'WPA2 + WPA3',
			detail: 'Works with everything, uses WPA3 where possible'
		},
		{ value: 'wpa3', label: 'WPA3 only', detail: 'Strongest, but older devices can’t join' },
		{ value: 'wpa2', label: 'WPA2 only', detail: 'For old devices that struggle with WPA3' },
		{
			value: 'enhanced-open',
			label: 'Open, encrypted',
			detail: 'No password, traffic is still encrypted'
		},
		{ value: 'open', label: 'Open', detail: 'No password and no encryption' }
	];

	let form = $state<WifiNetworkInput>(blank());
	let busy = $state<'save' | 'delete' | null>(null);
	let confirmDelete = $state(false);

	function blank(): WifiNetworkInput {
		return {
			ssid: '',
			security: 'wpa2-wpa3',
			password: '',
			hidden: false,
			isolate: false,
			network: 'lan',
			enabled: true,
			bands: [...new Set(radios.filter((radio) => radio.enabled).map((radio) => radio.band))]
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
					: { ...network, password: network.password ?? '', bands: [...network.bands] };
		});
	});

	const needsPassword = $derived(['wpa3', 'wpa2-wpa3', 'wpa2'].includes(form.security));

	async function save() {
		busy = 'save';
		try {
			const body = { ...form, password: needsPassword ? form.password : null };
			if (network === 'new') await api.post('/wifi/networks', body);
			else if (network) await api.put(`/wifi/networks/${network.id}`, body);
			toasts.show(network === 'new' ? `${form.ssid} is on the air` : 'Wi-Fi updated');
			onsaved();
			onclose();
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
			await api.delete(`/wifi/networks/${network.id}`);
			toasts.show(`${network.ssid} removed`);
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
	title={network === 'new' ? 'New network' : (network?.ssid ?? '')}
	{onclose}
>
	<div class="flex flex-col gap-6">
		{#if connectedHere}
			<p class="rounded-control bg-warn/10 px-4 py-3 text-sm text-warn">
				You're connected through this network. Saving will drop your connection for a few seconds,
				and you'll need the new name or password to rejoin.
			</p>
		{/if}

		<TextField bind:value={form.ssid} label="Name" placeholder="What devices see" maxlength={32} />

		<Select bind:value={form.security} options={securityOptions} label="Security" />

		{#if networkOptions.length > 1}
			<Select bind:value={form.network} options={networkOptions} label="Devices join" />
		{/if}

		{#if needsPassword}
			<TextField
				value={form.password ?? ''}
				oninput={(event) => (form.password = event.currentTarget.value)}
				label="Password"
				secret
				placeholder="At least 8 characters"
				maxlength={63}
			/>
		{/if}

		{#if bands.length > 1}
			<Segmented
				multiple
				bind:value={form.bands}
				label="Broadcast on"
				options={bands.map((band) => ({ value: band, label: bandLabel[band] }))}
			/>
		{/if}

		<div class="flex flex-col gap-5 pt-2">
			{@render option('Turned on', 'Broadcast this network.', 'enabled')}
			{@render option('Hidden', 'Devices have to type the name to join.', 'hidden')}
			{@render option(
				'Keep devices apart',
				'Devices can reach the internet but not each other.',
				'isolate'
			)}
		</div>
	</div>

	{#snippet footer()}
		{#if network !== 'new'}
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
		<Button
			variant="primary"
			onclick={save}
			busy={busy === 'save'}
			disabled={!form.ssid || !form.bands.length}
		>
			{network === 'new' ? 'Create' : 'Save'}
		</Button>
	{/snippet}
</Sheet>

{#snippet option(title: string, description: string, key: 'enabled' | 'hidden' | 'isolate')}
	<div class="flex items-center justify-between gap-6">
		<div>
			<p class="text-[15px]">{title}</p>
			<p class="text-[13px] text-muted">{description}</p>
		</div>
		<Toggle bind:checked={form[key]} label={title} />
	</div>
{/snippet}
