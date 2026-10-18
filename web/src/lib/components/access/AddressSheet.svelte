<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import type { Ddns } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Setting from '#lib/components/ui/Setting.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';

	type Props = { ddns: Ddns | null; open: boolean; onclose: () => void; onsaved: () => void };
	let { ddns, open, onclose, onsaved }: Props = $props();

	let enabled = $state(false);
	let hostname = $state('');
	let token = $state('');
	let busy = $state(false);

	$effect.pre(() => {
		if (!open) return;
		untrack(() => {
			enabled = ddns?.enabled ?? false;
			hostname = ddns?.hostname ?? '';
			token = '';
		});
	});

	const status = $derived.by(() => {
		const current = ddns?.status;
		if (!ddns?.enabled || !current) return null;
		if (current.error) return { tone: 'text-danger', text: current.error };
		if (!current.updated) return { tone: 'text-muted', text: 'Waiting for the first update.' };
		return {
			tone: 'text-accent',
			text: `Points at ${current.ipv4}${current.ipv6 ? ` and ${current.ipv6}` : ''}`
		};
	});

	async function save() {
		busy = true;
		try {
			await api.put('/ddns', { enabled, hostname, token: token || null });
			toasts.show(enabled ? `${hostname} follows your address now` : 'Your own address is off');
			setTimeout(onsaved, 3000);
			onclose();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<Sheet {open} title="Your own address" {onclose}>
	<div class="flex flex-col gap-5">
		<Setting
			title="Keep a hostname pointed at home"
			hint="Uses a domain on your Cloudflare account"
		>
			<Toggle bind:checked={enabled} label="Your own address" />
		</Setting>
		{#if enabled}
			<TextField bind:value={hostname} label="Hostname" placeholder="home.example.com" />
			<TextField
				bind:value={token}
				label="Cloudflare API token"
				placeholder={ddns?.hasToken ? 'Saved, type to replace' : 'With permission to edit DNS'}
				secret
			/>
			{#if status}<p class="text-[13px] {status.tone}">{status.text}</p>{/if}
		{/if}
	</div>

	{#snippet footer()}
		<Button onclick={onclose}>Cancel</Button>
		<Button variant="primary" onclick={save} {busy}>Save</Button>
	{/snippet}
</Sheet>
