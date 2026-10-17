<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import type { Acceleration, SmartQueue } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Block from '#lib/components/ui/Block.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Segmented from '#lib/components/ui/Segmented.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import Toggle from '#lib/components/ui/Toggle.svelte';

	type Props = { qos: SmartQueue; acceleration: Acceleration; onsaved: () => void };
	let { qos, acceleration, onsaved }: Props = $props();

	let enabled = $state(false);
	let download = $state('');
	let upload = $state('');
	let mode = $state<Acceleration>('off');
	let busy = $state(false);

	const key = $derived(`${qos.enabled}|${qos.download}|${qos.upload}|${acceleration}`);

	$effect.pre(() => {
		if (!key) return;
		untrack(() => {
			enabled = qos.enabled;
			download = qos.download ? String(qos.download / 1000) : '';
			upload = qos.upload ? String(qos.upload / 1000) : '';
			mode = acceleration;
		});
	});

	const kbit = (mbps: string) => Math.round(Number(mbps || 0) * 1000);
	const changed = $derived(
		enabled !== qos.enabled ||
			(enabled && (kbit(download) !== qos.download || kbit(upload) !== qos.upload))
	);

	async function saveQueue() {
		busy = true;
		try {
			await api.put('/internet/qos', { enabled, download: kbit(download), upload: kbit(upload) });
			toasts.show(enabled ? 'Smart queue is on' : 'Smart queue is off');
			onsaved();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}

	async function setMode(next: Acceleration) {
		try {
			await api.put('/internet/acceleration', { mode: next });
			toasts.show(next === 'off' ? 'Acceleration off' : 'Acceleration on');
			onsaved();
		} catch (err) {
			toasts.error(err);
		}
	}
</script>

<Block
	title="Smart queue"
	description="Keeps video calls and games smooth while something big downloads, and lets device priority work."
>
	{#snippet aside()}
		<Toggle bind:checked={enabled} label="Smart queue" />
	{/snippet}
	{#if enabled}
		<div class="flex flex-col gap-4">
			<div class="grid grid-cols-2 gap-3">
				<TextField
					bind:value={download}
					label="Download speed"
					placeholder="Mbps"
					inputmode="decimal"
				/>
				<TextField
					bind:value={upload}
					label="Upload speed"
					placeholder="Mbps"
					inputmode="decimal"
				/>
			</div>
			<p class="text-[13px] text-faint">
				Use about 90% of what your plan really delivers. Too high and it can't help, too low and you
				lose speed.
			</p>
		</div>
	{/if}
	{#if changed}
		<Button variant="primary" class="mt-4 w-full" onclick={saveQueue} {busy}>Save</Button>
	{/if}
</Block>

<Block
	title="Acceleration"
	description="Lets the router hand established connections straight through for higher speeds. While it's on, speed limits, priority and the smart queue don't apply to accelerated traffic."
>
	<Segmented
		bind:value={mode}
		onchange={setMode}
		options={[
			{ value: 'off', label: 'Off' },
			{ value: 'software', label: 'Software' },
			{ value: 'hardware', label: 'Hardware' }
		]}
	/>
</Block>
