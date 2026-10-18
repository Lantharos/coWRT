<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import type { DnsProvider, Protection } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Choices from '#lib/components/ui/controls/Choices.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';
	import { providers } from './providers';

	type Props = {
		current: Protection['privateDns'] | null;
		open: boolean;
		onclose: () => void;
		onsaved: () => void;
	};
	let { current, open, onclose, onsaved }: Props = $props();

	let provider = $state<DnsProvider>('off');
	let nextdns = $state('');
	let busy = $state(false);

	$effect.pre(() => {
		if (!open) return;
		untrack(() => {
			provider = current?.provider ?? 'off';
			nextdns = current?.nextdns ?? '';
		});
	});

	async function save() {
		busy = true;
		if (provider !== 'off') toasts.show('Setting up encrypted DNS');
		try {
			await api.put('/protection/dns', { provider, nextdns: nextdns.trim() || null });
			toasts.show(provider === 'off' ? 'Using your provider’s DNS' : 'Lookups are encrypted now');
			onsaved();
			onclose();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<Sheet {open} title="Encrypted DNS" {onclose}>
	<div class="flex flex-col gap-5">
		<p class="text-[14px] text-muted">
			Your internet provider can't see or change which sites you look up.
		</p>
		<Choices bind:value={provider} label="Encrypted DNS provider" options={providers} />
		{#if provider === 'nextdns'}
			<TextField bind:value={nextdns} label="NextDNS configuration ID" placeholder="abc123" />
		{/if}
	</div>

	{#snippet footer()}
		<Button onclick={onclose}>Cancel</Button>
		<Button variant="primary" onclick={save} {busy}>Save</Button>
	{/snippet}
</Sheet>
