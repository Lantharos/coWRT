<script lang="ts">
	import { api } from '#lib/api/client.js';
	import type { Radio, WifiScan } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';

	type Props = { radio: Radio; onpick: (channel: number) => void };
	let { radio, onpick }: Props = $props();

	let scan = $state<WifiScan | null>(null);
	let busy = $state(false);

	const current = $derived(radio.live?.channel ?? null);

	async function look() {
		busy = true;
		try {
			scan = await api.post<WifiScan>(`/wifi/radios/${radio.name}/scan`);
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<div class="flex flex-col gap-4">
	{#if scan}
		<div class="flex flex-col gap-2">
			<div class="flex h-24 items-end gap-[3px]">
				{#each scan.channels as channel (channel.channel)}
					<div
						class="min-w-0 flex-1 rounded-t-[4px] {channel.channel === scan.best
							? 'bg-accent'
							: channel.channel === current
								? 'bg-fg/60'
								: 'bg-overlay'}"
						style:height="{Math.max(6, channel.load * 100)}%"
						title="Channel {channel.channel}: {channel.networks} networks"
					></div>
				{/each}
			</div>
			<div class="flex gap-[3px] text-center text-[10px] text-faint">
				{#each scan.channels as channel, index (channel.channel)}
					<span class="min-w-0 flex-1 truncate"
						>{scan.channels.length <= 13 || index % 4 === 0 ? channel.channel : ''}</span
					>
				{/each}
			</div>
		</div>
		<div class="flex items-center justify-between gap-4">
			<p class="text-[14px] text-muted">
				{scan.neighbors.length}
				{scan.neighbors.length === 1 ? 'network' : 'networks'} nearby.
				{#if scan.best === current}You're on the quietest channel.{:else if scan.best}Channel {scan.best}
					is quietest.{/if}
			</p>
			{#if scan.best && scan.best !== current}
				<Button size="sm" onclick={() => onpick(scan!.best!)}>Use {scan.best}</Button>
			{/if}
		</div>
	{:else}
		<Button onclick={look} {busy}>
			{busy ? 'Listening to every channel…' : 'Look for a quieter channel'}
		</Button>
	{/if}
</div>
