<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import type { UpdateInfo } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';

	type Props = { current: string; open: boolean; onclose: () => void };
	let { current, open, onclose }: Props = $props();

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

	$effect(() => {
		if (open) untrack(check);
	});

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
</script>

<Sheet {open} title="Updates" {onclose}>
	<div class="flex flex-col gap-4">
		<p class="text-[15px]">
			{#if !info}
				Looking for updates…
			{:else if info.available}
				coWRT {info.latest} is out. You're on {info.current}.
			{:else}
				You're on the newest version, {info.current || current}.
			{/if}
		</p>
		{#if info?.available && info.notes}
			<p class="text-[14px] whitespace-pre-line text-muted">{info.notes}</p>
		{/if}
	</div>

	{#snippet footer()}
		{#if info?.available}
			<Button variant="primary" onclick={install} busy={busy === 'install'}>
				Install {info.latest}
			</Button>
		{:else}
			<Button onclick={check} busy={busy === 'check'}>Check again</Button>
		{/if}
	{/snippet}
</Sheet>
