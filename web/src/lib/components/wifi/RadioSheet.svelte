<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import type { Radio, RadioInput } from '#lib/api/types.js';
	import { bandLabel } from '#lib/format.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Segmented from '#lib/components/ui/controls/Segmented.svelte';
	import Select from '#lib/components/ui/controls/Select.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import Setting from '#lib/components/ui/Setting.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';
	import { countries } from './countries';

	type Props = { radio: Radio | null; onclose: () => void; onsaved: () => void };
	let { radio, onclose, onsaved }: Props = $props();

	const maxWidth: Record<string, number> = { '2g': 40, '5g': 160, '6g': 320, '60g': 2160 };
	const powers = [
		{ value: 'full', label: 'Full', dbm: null },
		{ value: 'reduced', label: 'Reduced', dbm: 17 },
		{ value: 'low', label: 'Low', dbm: 12 }
	] as const;
	type Power = (typeof powers)[number]['value'];

	let form = $state<RadioInput>({
		channel: 'auto',
		htmode: null,
		txpower: null,
		country: null,
		enabled: true
	});
	let power = $state<Power>('full');
	let width = $state('');
	let busy = $state(false);

	const key = $derived(radio?.name);

	$effect.pre(() => {
		if (!key) return;
		untrack(() => reset());
	});

	function reset() {
		if (!radio) return;
		form = {
			channel: radio.channel,
			htmode: radio.htmode,
			txpower: radio.txpower,
			country: radio.country,
			enabled: radio.enabled
		};
		power = powers.find((p) => p.dbm === radio.txpower)?.value ?? 'full';
		width = radio.htmode ?? '';
	}

	const family = $derived(radio?.htmode?.match(/^[A-Z]+/)?.[0] ?? 'HE');
	const widths = $derived(
		(radio?.htmodes ?? [])
			.filter((mode) => new RegExp(`^${family}\\d+$`).test(mode))
			.filter((mode) => Number(mode.slice(family.length)) <= (maxWidth[radio?.band ?? '5g'] ?? 160))
			.map((mode) => ({ value: mode, label: `${mode.slice(family.length)} MHz` }))
	);
	const channels = $derived([
		{ value: 'auto', label: 'Automatic', detail: 'Picks the quietest channel on startup' },
		...(radio?.channels ?? []).map((channel) => ({
			value: String(channel),
			label: `Channel ${channel}`
		}))
	]);

	async function save() {
		if (!radio) return;
		busy = true;
		try {
			const txpower = powers.find((p) => p.value === power)?.dbm ?? null;
			await api.put(`/wifi/radios/${radio.name}`, {
				...form,
				htmode: width || form.htmode,
				txpower
			});
			toasts.show(`${bandLabel[radio.band]} radio updated`);
			onsaved();
			onclose();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<Sheet open={radio !== null} title="{radio ? bandLabel[radio.band] : ''} radio" {onclose}>
	<div class="flex flex-col gap-6">
		<Setting title="Turned on">
			<Toggle bind:checked={form.enabled} label="Radio on" />
		</Setting>

		<Select bind:value={form.channel} options={channels} label="Channel" />

		{#if widths.length > 1 && width}
			<Segmented bind:value={width} options={widths} label="Channel width" />
			<p class="-mt-4 text-[13px] text-faint">Narrower holds up better through walls.</p>
		{/if}

		<Segmented
			bind:value={power}
			options={powers.map(({ value, label }) => ({ value, label }))}
			label="Transmit power"
		/>

		<Select
			value={form.country ?? 'auto'}
			onchange={(value) => (form.country = value === 'auto' ? null : value)}
			options={[{ value: 'auto', label: 'Not set' }, ...countries]}
			label="Country"
		/>
	</div>

	{#snippet footer()}
		<Button onclick={onclose}>Cancel</Button>
		<Button variant="primary" onclick={save} {busy}>Save</Button>
	{/snippet}
</Sheet>
