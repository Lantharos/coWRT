<script lang="ts">
	import { api } from '#lib/api/client.js';
	import type { Upnp } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Block from '#lib/components/ui/Block.svelte';
	import Toggle from '#lib/components/ui/Toggle.svelte';

	let { upnp, onsaved }: { upnp: Upnp; onsaved: () => void } = $props();

	let busy = $state(false);

	async function toggle(enabled: boolean) {
		busy = true;
		if (enabled && !upnp.installed) toasts.show('Installing UPnP, this takes a moment');
		try {
			await api.put('/upnp', { enabled });
			toasts.show(enabled ? 'Apps can open ports now' : 'Automatic ports are off');
			onsaved();
		} catch (err) {
			toasts.error(err);
			onsaved();
		} finally {
			busy = false;
		}
	}
</script>

<Block
	title="Automatic port forwarding"
	description="Lets games and apps on trusted networks open the ports they need (UPnP and NAT-PMP)."
>
	{#snippet aside()}
		<Toggle
			checked={upnp.enabled}
			label="Automatic port forwarding"
			disabled={busy}
			onchange={toggle}
		/>
	{/snippet}
	{#if upnp.enabled}
		{#if upnp.mappings.length}
			<div class="flex flex-col gap-1.5 text-[15px]">
				{#each upnp.mappings as mapping (`${mapping.protocol}${mapping.external}`)}
					<div class="flex justify-between gap-4">
						<span class="truncate">{mapping.description || 'Unnamed app'}</span>
						<span class="shrink-0 text-muted">
							{mapping.protocol}
							{mapping.external} → {mapping.address}:{mapping.internal}
						</span>
					</div>
				{/each}
			</div>
		{:else}
			<p class="text-sm text-muted">No app has opened a port yet.</p>
		{/if}
	{/if}
</Block>
