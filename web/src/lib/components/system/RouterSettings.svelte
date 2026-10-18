<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import type { SystemInfo } from '#lib/api/types.js';
	import timezones from '#lib/data/timezones.json';
	import { toasts } from '#lib/toast.svelte.js';
	import Block from '#lib/components/ui/Block.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Select from '#lib/components/ui/Select.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';

	let { system, onsaved }: { system: SystemInfo; onsaved: () => void } = $props();

	let hostname = $state('');
	let zone = $state('UTC');
	let busy = $state(false);

	const key = $derived(`${system.hostname}|${system.zonename}`);

	$effect.pre(() => {
		if (!key) return;
		untrack(() => {
			hostname = system.hostname;
			zone = system.zonename ?? 'UTC';
		});
	});

	const options = timezones.map(({ zone }) => ({ value: zone, label: zone.replaceAll('_', ' ') }));
	const changed = $derived(hostname !== system.hostname || zone !== (system.zonename ?? 'UTC'));

	async function save() {
		busy = true;
		const rule = timezones.find((entry) => entry.zone === zone)?.rule ?? 'GMT0';
		try {
			await api.put('/system/settings', { hostname, zonename: zone, timezone: rule });
			toasts.show('Saved');
			onsaved();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<Block>
	<div class="flex flex-col gap-4">
		<TextField bind:value={hostname} label="Router name" maxlength={63} />
		<Select bind:value={zone} {options} label="Time zone" />
		<p class="-mt-2 text-[13px] text-faint">Quiet times and daily totals follow this time zone.</p>
		{#if changed}
			<Button variant="primary" onclick={save} {busy}>Save</Button>
		{/if}
	</div>
</Block>
