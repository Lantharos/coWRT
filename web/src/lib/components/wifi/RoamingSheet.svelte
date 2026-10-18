<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import type { Roaming } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';
	import Setting from '#lib/components/ui/Setting.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';

	type Props = { roaming: Roaming | null; open: boolean; onclose: () => void; onsaved: () => void };
	let { roaming, open, onclose, onsaved }: Props = $props();

	let fast = $state(false);
	let steering = $state(false);
	let busy = $state(false);

	$effect.pre(() => {
		if (!open) return;
		untrack(() => {
			fast = roaming?.fast ?? false;
			steering = roaming?.steering ?? false;
		});
	});

	async function save() {
		busy = true;
		if (steering && !roaming?.steering)
			toasts.show('Installing band steering, this takes a moment');
		try {
			await api.put('/wifi/roaming', { fast, steering });
			toasts.show('Roaming updated');
			onsaved();
			onclose();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<Sheet {open} title="Roaming" {onclose}>
	<div class="flex flex-col">
		<Setting title="Fast roaming" hint="Calls don't drop when moving between access points">
			<Toggle bind:checked={fast} label="Fast roaming" />
		</Setting>
		<Setting title="Band steering" hint="Nudges devices onto the fastest band">
			<Toggle bind:checked={steering} label="Band steering" />
		</Setting>
	</div>

	{#snippet footer()}
		<Button onclick={onclose}>Cancel</Button>
		<Button variant="primary" onclick={save} {busy}>Save</Button>
	{/snippet}
</Sheet>
