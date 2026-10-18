<script lang="ts">
	import { Trash2 } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import type { Uplink } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import { uplinkState } from './uplinks';

	type Props = { phone: Uplink | null; open: boolean; onclose: () => void; onsaved: () => void };
	let { phone, open, onclose, onsaved }: Props = $props();

	let busy = $state<'setup' | 'use' | 'remove' | null>(null);
	let confirmRemove = $state(false);

	async function run(
		task: NonNullable<typeof busy>,
		request: () => Promise<unknown>,
		message: string
	) {
		busy = task;
		try {
			await request();
			toasts.show(message);
			onsaved();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}

	function remove() {
		if (!confirmRemove) {
			confirmRemove = true;
			return;
		}
		run('remove', () => api.delete(`/internet/uplinks/${phone?.id}`), 'Phone removed').then(
			onclose
		);
	}
</script>

<Sheet {open} title="Phone as backup" onclose={() => ((confirmRemove = false), onclose())}>
	<div class="flex flex-col gap-5 text-[15px]">
		{#if phone}
			<p class={phone.online ? 'text-accent' : 'text-muted'}>
				{uplinkState(phone)}{#if phone.latencyMs !== null && phone.online}, {phone.latencyMs} ms{/if}
			</p>
		{/if}
		<ol class="flex list-decimal flex-col gap-2 pl-5 text-muted marker:text-faint">
			<li>Plug your phone into the router's USB port.</li>
			<li>Android: turn on USB tethering. iPhone: turn on Personal Hotspot and tap Trust.</li>
			<li>The router picks it up by itself and uses it when the main line goes down.</li>
		</ol>
	</div>

	{#snippet footer()}
		{#if phone}
			<Button
				variant={confirmRemove ? 'danger' : 'ghost'}
				class="mr-auto"
				onclick={remove}
				busy={busy === 'remove'}
			>
				<Trash2 size={16} />
				{confirmRemove ? 'Really remove?' : 'Remove'}
			</Button>
			{#if phone.online && !phone.inUse}
				<Button
					onclick={() =>
						run(
							'use',
							() => api.put('/internet/failover', { mode: phone.id, failback: true }),
							'Using your phone now'
						)}
					busy={busy === 'use'}
				>
					Use it now
				</Button>
			{/if}
		{:else}
			<Button onclick={onclose}>Cancel</Button>
			<Button
				variant="primary"
				onclick={() =>
					run('setup', () => api.post('/internet/uplinks/phone'), 'Ready for your phone')}
				busy={busy === 'setup'}
			>
				Set up
			</Button>
		{/if}
	{/snippet}
</Sheet>
