<script lang="ts">
	import { api } from '#lib/api/client.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Select from '#lib/components/ui/controls/Select.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';
	import { countries } from '#lib/components/wifi/countries.js';
	import { generatePassword } from '#lib/components/wifi/passwords.js';

	let { ondone }: { ondone: () => void } = $props();

	const region = new Intl.Locale(navigator.language).maximize().region ?? 'US';

	let ssid = $state('');
	let password = $state(generatePassword());
	let country = $state(countries.some((c) => c.value === region) ? region : 'US');
	let busy = $state(false);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		busy = true;
		try {
			await api.put('/setup/wifi', { ssid: ssid.trim(), password, country });
			ondone();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<form class="flex flex-col gap-3" onsubmit={submit}>
	<TextField bind:value={ssid} label="Wi-Fi name" placeholder="Home" maxlength={32} autofocus />
	<TextField bind:value={password} label="Wi-Fi password" maxlength={63} />
	<Select bind:value={country} options={countries} label="Country" />
	<Button
		variant="primary"
		type="submit"
		class="mt-3 h-11"
		{busy}
		disabled={!ssid.trim() || password.length < 8}
	>
		Continue
	</Button>
	<Button variant="ghost" onclick={ondone}>Skip Wi-Fi for now</Button>
</form>
