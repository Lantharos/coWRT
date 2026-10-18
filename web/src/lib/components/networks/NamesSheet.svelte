<script lang="ts">
	import { api } from '#lib/api/client.js';
	import type { Device, LocalName } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Select from '#lib/components/ui/controls/Select.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import EntryList from './EntryList.svelte';

	type Props = {
		names: LocalName[];
		domain: string | null;
		devices: Device[];
		open: boolean;
		onclose: () => void;
		onsaved: () => void;
	};
	let { names, domain, devices, open, onclose, onsaved }: Props = $props();

	const OTHER = 'other';

	let name = $state('');
	let target = $state(OTHER);
	let address = $state('');
	let busy = $state(false);

	const addressable = $derived(devices.filter((d) => d.reservedIp ?? d.ipv4));
	const targets = $derived([
		...addressable.map((d) => ({
			value: d.reservedIp ?? d.ipv4!,
			label: d.name,
			detail: d.reservedIp ?? d.ipv4!
		})),
		{ value: OTHER, label: 'Another address' }
	]);
	const ip = $derived(target === OTHER ? address.trim() : target);
	const owner = (entry: LocalName) =>
		devices.find((d) => (d.reservedIp ?? d.ipv4) === entry.ip)?.name;
	const full = (entry: LocalName) =>
		domain && !entry.name.includes('.') ? `${entry.name}.${domain}` : entry.name;

	async function change(task: () => Promise<unknown>, message: string) {
		busy = true;
		try {
			await task();
			toasts.show(message);
			onsaved();
			return true;
		} catch (err) {
			toasts.error(err);
			return false;
		} finally {
			busy = false;
		}
	}

	async function add(event: SubmitEvent) {
		event.preventDefault();
		const added = await change(
			() => api.post('/lan/names', { name: name.trim(), ip }),
			`${name.trim()} points at ${ip}`
		);
		if (added) name = address = '';
	}
</script>

<Sheet {open} title="Local names" {onclose}>
	<div class="flex flex-col gap-6">
		<p class="text-[14px] text-muted">Reach things at home by name instead of their address.</p>
		<form class="flex flex-col gap-3" onsubmit={add}>
			<TextField bind:value={name} label="Name" placeholder="nas" />
			<Select bind:value={target} options={targets} label="Points at" />
			{#if target === OTHER}
				<TextField bind:value={address} label="Address" placeholder="10.0.0.50" />
			{/if}
			<Button type="submit" variant="primary" {busy} disabled={!name.trim() || !ip}>Add name</Button
			>
		</form>
		<EntryList
			items={names}
			empty="No names yet."
			label={full}
			{busy}
			onremove={(entry) =>
				change(() => api.delete(`/lan/names/${entry.id}`), `${full(entry)} removed`)}
		>
			{#snippet children(entry)}
				<p class="truncate">{full(entry)}</p>
				<p class="truncate text-[13px] text-muted">{owner(entry) ?? entry.ip}</p>
			{/snippet}
		</EntryList>
	</div>
</Sheet>
