<script lang="ts">
	import { Download, RotateCcw } from '@lucide/svelte';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import FilePicker from './FilePicker.svelte';
	import { send } from './recovery';

	let { open, onclose }: { open: boolean; onclose: () => void } = $props();

	let file = $state<File | null>(null);
	let busy = $state(false);

	async function restore() {
		if (!file) return;
		busy = true;
		try {
			await send('restore', file);
		} catch (err) {
			toasts.error(err);
			busy = false;
		}
	}
</script>

<Sheet {open} title="Backups" onclose={() => ((file = null), onclose())}>
	<div class="flex flex-col gap-3">
		<p class="pb-2 text-[14px] text-muted">
			Everything you've set up, in one file. Keep a copy before big changes.
		</p>
		<a
			href="/api/system/backup"
			download
			class="inline-flex h-11 items-center justify-center gap-2 rounded-control bg-accent px-4 text-[15px] font-medium text-[#06130a] transition-colors hover:bg-accent-strong"
		>
			<Download size={17} /> Download backup
		</a>
		<FilePicker accept=".tar.gz,.tgz" onpick={(picked) => (file = picked)}>
			<RotateCcw size={16} /> Restore from a file
		</FilePicker>
		{#if file}
			<div class="mt-3 flex flex-col gap-4 rounded-[16px] bg-raised p-4">
				<p class="text-[15px]">Restore {file.name}? The router restarts afterwards.</p>
				<div class="flex justify-end gap-2">
					<Button variant="ghost" onclick={() => (file = null)}>Cancel</Button>
					<Button variant="danger" onclick={restore} {busy}>Restore</Button>
				</div>
			</div>
		{/if}
	</div>
</Sheet>
