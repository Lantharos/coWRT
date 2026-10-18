<script lang="ts">
	import { Download, HardDriveUpload, Power, RotateCcw } from '@lucide/svelte';
	import { api, ApiError } from '#lib/api/client.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Block from '#lib/components/ui/Block.svelte';
	import Button from '#lib/components/ui/Button.svelte';

	type Upload = { kind: 'restore' | 'firmware'; file: File };

	let picker = $state<HTMLInputElement>();
	let picking = $state<Upload['kind']>('restore');
	let pending = $state<Upload | null>(null);
	let busy = $state<string | null>(null);
	let confirmRestart = $state(false);

	function pick(kind: Upload['kind']) {
		picking = kind;
		picker?.click();
	}

	function picked(event: Event) {
		const file = (event.currentTarget as HTMLInputElement).files?.[0];
		if (file) pending = { kind: picking, file };
		(event.currentTarget as HTMLInputElement).value = '';
	}

	async function waitForRouter() {
		toasts.show('Restarting. This page comes back by itself.');
		await new Promise((resolve) => setTimeout(resolve, 20000));
		for (;;) {
			const alive = await fetch('/api/auth/session')
				.then((r) => r.ok)
				.catch(() => false);
			if (alive) return location.reload();
			await new Promise((resolve) => setTimeout(resolve, 3000));
		}
	}

	async function upload() {
		if (!pending) return;
		busy = pending.kind;
		try {
			const response = await fetch(`/api/system/${pending.kind}`, {
				method: 'POST',
				body: pending.file
			});
			if (!response.ok) {
				const payload = await response.json().catch(() => null);
				throw new ApiError(response.status, payload?.error ?? 'The router refused the file');
			}
			pending = null;
			waitForRouter();
		} catch (err) {
			toasts.error(err);
			busy = null;
		}
	}

	async function restart() {
		if (!confirmRestart) {
			confirmRestart = true;
			return;
		}
		busy = 'restart';
		try {
			await api.post('/system/reboot');
			waitForRouter();
		} catch (err) {
			toasts.error(err);
			busy = null;
		}
	}

	const describe = $derived(
		pending?.kind === 'restore'
			? `Restore the settings in ${pending.file.name}? The router restarts afterwards.`
			: pending
				? `Install ${pending.file.name}? Your settings are kept, but anything not in the file, including coWRT itself if it isn't built in, is removed. The router restarts and is offline for a few minutes.`
				: ''
	);
</script>

<input
	bind:this={picker}
	type="file"
	class="hidden"
	accept={picking === 'restore' ? '.tar.gz,.tgz' : '.bin,.img,.itb'}
	onchange={picked}
/>

<Block
	title="Backups"
	description="Everything you've set up, in one file. Keep a copy before big changes."
>
	<div class="grid gap-2 sm:grid-cols-2">
		<a
			href="/api/system/backup"
			download
			class="inline-flex h-10 items-center justify-center gap-2 rounded-control bg-raised px-4 text-sm font-medium transition-colors hover:bg-overlay"
		>
			<Download size={16} /> Download backup
		</a>
		<Button onclick={() => pick('restore')}><RotateCcw size={16} /> Restore from a file</Button>
	</div>
</Block>

<Block title="Firmware" description="Install an OpenWrt image for this router. Settings are kept.">
	<Button onclick={() => pick('firmware')} class="w-full">
		<HardDriveUpload size={16} /> Choose firmware file
	</Button>
</Block>

{#if pending}
	<Block class="bg-raised">
		<div class="flex flex-col gap-4">
			<p class="text-[15px]">{describe}</p>
			<div class="flex justify-end gap-2">
				<Button variant="ghost" onclick={() => (pending = null)}>Cancel</Button>
				<Button variant="danger" onclick={upload} busy={busy === pending.kind}>
					{pending.kind === 'restore' ? 'Restore' : 'Install'}
				</Button>
			</div>
		</div>
	</Block>
{/if}

<Block
	title="Restart"
	description="Turns the router off and on again. Everything is offline for about a minute."
>
	{#snippet aside()}
		<Button
			variant={confirmRestart ? 'danger' : 'subtle'}
			size="sm"
			onclick={restart}
			busy={busy === 'restart'}
		>
			<Power size={15} />
			{confirmRestart ? 'Really restart?' : 'Restart'}
		</Button>
	{/snippet}
</Block>
