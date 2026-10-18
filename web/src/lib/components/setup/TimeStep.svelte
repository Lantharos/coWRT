<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '#lib/api/client.js';
	import type { SystemInfo } from '#lib/api/types.js';
	import timezones from '#lib/data/timezones.json';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Select from '#lib/components/ui/controls/Select.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';

	let { ondone }: { ondone: () => void } = $props();

	const options = timezones.map(({ zone }) => ({ value: zone, label: zone.replaceAll('_', ' ') }));
	const guessed = Intl.DateTimeFormat().resolvedOptions().timeZone;

	let hostname = $state('');
	let zone = $state(timezones.some((entry) => entry.zone === guessed) ? guessed : 'UTC');
	let busy = $state(false);

	onMount(async () => {
		hostname = (await api.get<SystemInfo>('/system').catch(() => null))?.hostname ?? 'coWRT';
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		busy = true;
		const rule = timezones.find((entry) => entry.zone === zone)?.rule ?? 'GMT0';
		try {
			await api.put('/system/settings', {
				hostname: hostname.trim(),
				zonename: zone,
				timezone: rule
			});
			await api.post('/setup/done');
			ondone();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<form class="flex flex-col gap-3" onsubmit={submit}>
	<TextField bind:value={hostname} label="Router name" maxlength={63} />
	<Select bind:value={zone} {options} label="Time zone" />
	<Button variant="primary" type="submit" class="mt-3 h-11" {busy} disabled={!hostname.trim()}>
		Finish
	</Button>
</form>
