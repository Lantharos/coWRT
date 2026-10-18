<script lang="ts">
	import { api } from '#lib/api/client.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';

	let { open, onclose }: { open: boolean; onclose: () => void } = $props();

	let current = $state('');
	let next = $state('');
	let repeat = $state('');
	let busy = $state(false);

	const mismatch = $derived(repeat.length > 0 && next !== repeat);

	function close() {
		current = next = repeat = '';
		onclose();
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		busy = true;
		try {
			await api.put('/system/password', { current, next });
			toasts.show('Password changed');
			close();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<Sheet {open} title="Password" onclose={close}>
	<form id="password" class="flex flex-col gap-4" onsubmit={save}>
		<p class="text-[14px] text-muted">For signing in here and over SSH. Wi-Fi passwords stay.</p>
		<TextField bind:value={current} label="Current password" secret />
		<TextField bind:value={next} label="New password" secret hint="At least 8 characters" />
		<TextField
			bind:value={repeat}
			label="Again"
			secret
			hint={mismatch ? "These don't match" : undefined}
		/>
	</form>

	{#snippet footer()}
		<Button onclick={close}>Cancel</Button>
		<Button
			variant="primary"
			type="submit"
			form="password"
			{busy}
			disabled={!current || next.length < 8 || next !== repeat}
		>
			Change password
		</Button>
	{/snippet}
</Sheet>
