<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import type { Ddns } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Block from '#lib/components/ui/Block.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import Toggle from '#lib/components/ui/Toggle.svelte';

	let { ddns, onsaved }: { ddns: Ddns; onsaved: () => void } = $props();

	let enabled = $state(false);
	let hostname = $state('');
	let token = $state('');
	let busy = $state(false);

	const key = $derived(`${ddns.enabled}|${ddns.hostname}`);

	$effect.pre(() => {
		if (!key) return;
		untrack(() => {
			enabled = ddns.enabled;
			hostname = ddns.hostname;
			token = '';
		});
	});

	const changed = $derived(enabled !== ddns.enabled || hostname !== ddns.hostname || token !== '');
	const status = $derived.by(() => {
		const { status } = ddns;
		if (status.error) return { tone: 'text-danger', text: status.error };
		if (!status.updated) return { tone: 'text-muted', text: 'Waiting for the first update.' };
		const minutes = Math.round((Date.now() / 1000 - status.updated) / 60);
		return {
			tone: 'text-accent',
			text: `Points at ${status.ipv4}${status.ipv6 ? ` and ${status.ipv6}` : ''}, checked ${minutes < 1 ? 'just now' : `${minutes} min ago`}.`
		};
	});

	async function save() {
		busy = true;
		try {
			await api.put('/ddns', { enabled, hostname, token: token || null });
			toasts.show(enabled ? `${hostname} will follow your address` : 'Dynamic DNS is off');
			setTimeout(onsaved, 3000);
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<Block
	title="Your own address"
	description="Keeps a hostname on your Cloudflare domain pointed at home, even when your provider changes your IP."
>
	{#snippet aside()}
		<Toggle bind:checked={enabled} label="Dynamic DNS" />
	{/snippet}
	{#if enabled}
		<div class="flex flex-col gap-4">
			<TextField bind:value={hostname} label="Hostname" placeholder="home.example.com" />
			<TextField
				bind:value={token}
				label="Cloudflare API token"
				placeholder={ddns.hasToken ? 'Saved, type to replace' : 'With DNS edit permission'}
				secret
			/>
			{#if ddns.enabled}
				<p class="text-[13px] {status.tone}">{status.text}</p>
			{/if}
		</div>
	{/if}
	{#if changed}
		<Button variant="primary" class="mt-4 w-full" onclick={save} {busy}>Save</Button>
	{/if}
</Block>
