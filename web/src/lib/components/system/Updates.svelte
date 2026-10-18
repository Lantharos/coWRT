<script lang="ts">
	import { api } from '#lib/api/client.js';
	import type { UpdateInfo } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Block from '#lib/components/ui/Block.svelte';
	import Button from '#lib/components/ui/Button.svelte';

	let { current }: { current: string } = $props();

	let info = $state<UpdateInfo | null>(null);
	let busy = $state<'check' | 'install' | null>(null);

	async function check() {
		busy = 'check';
		try {
			info = await api.get<UpdateInfo>('/system/update');
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}

	async function install() {
		busy = 'install';
		try {
			await api.post('/system/update');
			toasts.show('Installing. The page reconnects by itself in a moment.');
			setTimeout(() => location.reload(), 15000);
		} catch (err) {
			toasts.error(err);
			busy = null;
		}
	}

	const description = $derived(
		!info
			? `You're on coWRT ${current}.`
			: info.available
				? `coWRT ${info.latest} is out. You're on ${info.current}.`
				: `You're on the newest version, ${info.current}.`
	);
</script>

<Block title="Updates" {description}>
	{#snippet aside()}
		{#if info?.available}
			<Button variant="primary" size="sm" onclick={install} busy={busy === 'install'}>
				Install {info.latest}
			</Button>
		{:else}
			<Button size="sm" onclick={check} busy={busy === 'check'}>Check now</Button>
		{/if}
	{/snippet}
	{#if info?.available && info.notes}
		<p class="text-sm whitespace-pre-line text-muted">{info.notes}</p>
	{/if}
</Block>
