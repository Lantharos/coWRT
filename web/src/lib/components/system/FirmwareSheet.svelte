<script lang="ts">
	import { HardDriveUpload } from '@lucide/svelte';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import FilePicker from './FilePicker.svelte';
	import { send } from './recovery';

	let { open, onclose }: { open: boolean; onclose: () => void } = $props();

	let file = $state<File | null>(null);
	let busy = $state(false);

	async function install() {
		if (!file) return;
		busy = true;
		try {
			await send('firmware', file);
		} catch (err) {
			toasts.error(err);
			busy = false;
		}
	}
</script>

<Sheet {open} title="Firmware" onclose={() => ((file = null), onclose())}>
	<div class="flex flex-col gap-3">
		<p class="pb-2 text-[14px] text-muted">
			Install an OpenWrt image for this router. Your settings are kept.
		</p>
		<FilePicker accept=".bin,.img,.itb" onpick={(picked) => (file = picked)}>
			<HardDriveUpload size={16} /> Choose firmware file
		</FilePicker>
		{#if file}
			<div class="mt-3 flex flex-col gap-4 rounded-[16px] bg-raised p-4">
				<p class="text-[15px]">
					Install {file.name}? Anything not in the image is removed, including coWRT if it isn't
					built in. The router is offline for a few minutes.
				</p>
				<div class="flex justify-end gap-2">
					<Button variant="ghost" onclick={() => (file = null)}>Cancel</Button>
					<Button variant="danger" onclick={install} {busy}>Install</Button>
				</div>
			</div>
		{/if}
	</div>
</Sheet>
