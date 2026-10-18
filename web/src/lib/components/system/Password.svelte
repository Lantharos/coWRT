<script lang="ts">
	import { api } from '#lib/api/client.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Block from '#lib/components/ui/Block.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';

	let current = $state('');
	let next = $state('');
	let repeat = $state('');
	let busy = $state(false);

	const mismatch = $derived(repeat.length > 0 && next !== repeat);

	async function save(event: SubmitEvent) {
		event.preventDefault();
		busy = true;
		try {
			await api.put('/system/password', { current, next });
			toasts.show('Password changed');
			current = next = repeat = '';
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<Block
	title="Password"
	description="Used to sign in here and over SSH. Changing it doesn't touch your Wi-Fi passwords."
>
	<form class="flex flex-col gap-4" onsubmit={save}>
		<TextField bind:value={current} label="Current password" secret />
		<div class="grid gap-4 sm:grid-cols-2">
			<TextField bind:value={next} label="New password" secret />
			<TextField
				bind:value={repeat}
				label="Again"
				secret
				hint={mismatch ? "These don't match" : undefined}
			/>
		</div>
		<Button
			variant="primary"
			type="submit"
			{busy}
			disabled={!current || next.length < 8 || next !== repeat}
		>
			Change password
		</Button>
	</form>
</Block>
