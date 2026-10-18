<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import type { SystemInfo } from '#lib/api/types.js';
	import timezones from '#lib/data/timezones.json';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Select from '#lib/components/ui/controls/Select.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';

	type Props = {
		system: SystemInfo | null;
		open: boolean;
		onclose: () => void;
		onsaved: () => void;
	};
	let { system, open, onclose, onsaved }: Props = $props();

	let hostname = $state('');
	let zone = $state('UTC');
	let busy = $state(false);

	$effect.pre(() => {
		if (!open) return;
		untrack(() => {
			hostname = system?.hostname ?? '';
			zone = system?.zonename ?? 'UTC';
		});
	});

	const options = timezones.map(({ zone }) => ({ value: zone, label: zone.replaceAll('_', ' ') }));
	const facts = $derived(
		system
			? [
					['Model', system.model],
					['Processor', system.processor],
					['coWRT', system.cowrt],
					['OpenWrt', `${system.openwrt} (${system.revision})`],
					['Kernel', system.kernel],
					['Platform', system.target]
				]
			: []
	);

	async function save() {
		busy = true;
		const rule = timezones.find((entry) => entry.zone === zone)?.rule ?? 'GMT0';
		try {
			await api.put('/system/settings', { hostname, zonename: zone, timezone: rule });
			toasts.show('Saved');
			onsaved();
			onclose();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<Sheet {open} title="This router" {onclose}>
	<div class="flex flex-col gap-8">
		<div class="flex flex-col gap-4">
			<TextField bind:value={hostname} label="Name" maxlength={63} />
			<Select bind:value={zone} {options} label="Time zone" />
		</div>
		<dl class="flex flex-col">
			{#each facts as [label, value] (label)}
				<div class="flex justify-between gap-6 py-2 text-[14px]">
					<dt class="text-muted">{label}</dt>
					<dd class="truncate text-right">{value}</dd>
				</div>
			{/each}
		</dl>
	</div>

	{#snippet footer()}
		<Button onclick={onclose}>Cancel</Button>
		<Button variant="primary" onclick={save} {busy}>Save</Button>
	{/snippet}
</Sheet>
