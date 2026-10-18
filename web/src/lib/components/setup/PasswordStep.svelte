<script lang="ts">
	import { api } from '#lib/api/client.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';

	let { ondone }: { ondone: () => void } = $props();

	let password = $state('');
	let repeat = $state('');
	let busy = $state(false);

	const mismatch = $derived(repeat.length > 0 && password !== repeat);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		busy = true;
		try {
			await api.post('/setup/password', { password });
			ondone();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<form class="flex flex-col gap-3" onsubmit={submit}>
	<TextField bind:value={password} label="Password" secret hint="At least 8 characters" autofocus />
	<TextField
		bind:value={repeat}
		label="Again"
		secret
		hint={mismatch ? "These don't match" : undefined}
	/>
	<Button
		variant="primary"
		type="submit"
		class="mt-3 h-11"
		{busy}
		disabled={password.length < 8 || password !== repeat}
	>
		Continue
	</Button>
</form>
