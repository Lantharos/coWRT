<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import type { UplinkStatus } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Choices from '#lib/components/ui/controls/Choices.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';
	import Setting from '#lib/components/ui/Setting.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';

	type Props = {
		status: UplinkStatus | null;
		open: boolean;
		onclose: () => void;
		onsaved: () => void;
	};
	let { status, open, onclose, onsaved }: Props = $props();

	let mode = $state('auto');
	let failback = $state(true);
	let busy = $state(false);

	$effect.pre(() => {
		if (!open) return;
		untrack(() => {
			mode = status?.mode ?? 'auto';
			failback = status?.failback ?? true;
		});
	});

	const main = $derived(status?.uplinks[0]);
	const backups = $derived(
		status?.uplinks
			.slice(1)
			.map((u) => u.name.toLowerCase())
			.join(' or ') ?? ''
	);
	const options = $derived([
		{
			value: 'auto',
			label: 'Automatic',
			detail: `Uses ${main?.name.toLowerCase() ?? 'the main line'}, moves to your ${backups} when it goes down`
		},
		...(status?.uplinks ?? []).map((uplink) => ({
			value: uplink.id,
			label: `Always ${uplink.name.toLowerCase()}`,
			detail: uplink.online ? undefined : 'Falls back to the others while it has no internet'
		}))
	]);

	async function save() {
		busy = true;
		try {
			await api.put('/internet/failover', { mode, failback });
			toasts.show(mode === 'auto' ? 'Switching automatically' : 'Switched');
			onsaved();
			onclose();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<Sheet {open} title="Failover" {onclose}>
	<div class="flex flex-col gap-6">
		<Choices bind:value={mode} label="Which connection to use" {options} />
		{#if mode === 'auto'}
			<Setting
				title="Switch back automatically"
				hint="Once {main?.name.toLowerCase() ?? 'the main line'} has been steady for a minute"
			>
				<Toggle bind:checked={failback} label="Switch back automatically" />
			</Setting>
		{/if}
	</div>

	{#snippet footer()}
		<Button onclick={onclose}>Cancel</Button>
		<Button variant="primary" onclick={save} {busy}>Save</Button>
	{/snippet}
</Sheet>
