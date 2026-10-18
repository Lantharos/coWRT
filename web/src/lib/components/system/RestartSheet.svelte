<script lang="ts">
	import { Power } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import { waitForRouter } from './recovery';

	let { open, onclose }: { open: boolean; onclose: () => void } = $props();

	let busy = $state(false);

	async function restart() {
		busy = true;
		try {
			await api.post('/system/reboot');
			waitForRouter();
		} catch (err) {
			toasts.error(err);
			busy = false;
		}
	}
</script>

<Sheet {open} title="Restart" {onclose}>
	<p class="text-[14px] text-muted">Everything is offline for about a minute.</p>

	{#snippet footer()}
		<Button onclick={onclose}>Cancel</Button>
		<Button variant="danger" onclick={restart} {busy}><Power size={16} /> Restart</Button>
	{/snippet}
</Sheet>
