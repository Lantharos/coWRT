<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import { applyChange } from '#lib/api/apply.js';
	import { live } from '#lib/api/live.svelte.js';
	import type { Acceleration, Applied, InternetOverview, QueueMode } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Segmented from '#lib/components/ui/controls/Segmented.svelte';
	import Select from '#lib/components/ui/controls/Select.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';
	import Setting from '#lib/components/ui/Setting.svelte';

	type Props = { settings: InternetOverview; onsaved: () => void };
	let { settings, onsaved }: Props = $props();

	const initial = untrack(() => settings);
	let queue = $state(initial.qos.enabled);
	let mode = $state<QueueMode>(initial.qos.mode);
	let download = $state(initial.qos.download ? String(initial.qos.download / 1000) : '');
	let upload = $state(initial.qos.upload ? String(initial.qos.upload / 1000) : '');
	let ipv6 = $state(initial.ipv6.enabled);
	let prefixSize = $state(initial.ipv6.prefix ? String(initial.ipv6.prefix) : 'auto');
	let busy = $state<'ipv6' | 'queue' | null>(null);

	const kbit = (mbps: string) => Math.round(Number(mbps || 0) * 1000);
	const modeHint = $derived<Record<QueueMode, string>>({
		adaptive: 'Keeps adjusting to what your line gives right now, up to your plan',
		speedtest:
			settings.qos.mode === 'speedtest' && settings.qos.download
				? `Set by the last speed test to ${Math.round(settings.qos.download / 1000)} / ${Math.round(settings.qos.upload / 1000)} Mbps`
				: 'Set by the speed test, then left alone',
		fixed: 'A little under what your line reliably gives'
	});
	const queueChanged = $derived(
		queue !== settings.qos.enabled ||
			(queue && mode !== settings.qos.mode) ||
			(queue &&
				mode !== 'speedtest' &&
				(kbit(download) !== settings.qos.download || kbit(upload) !== settings.qos.upload))
	);

	const ipv6Changed = $derived(
		ipv6 !== settings.ipv6.enabled ||
			(ipv6 && prefixSize !== (settings.ipv6.prefix ? String(settings.ipv6.prefix) : 'auto'))
	);
	const ipv6Status = $derived.by(() => {
		const wan = live.snapshot?.wan;
		if (!ipv6) return undefined;
		if (wan?.prefix6) return `Got ${wan.prefix6}`;
		if (wan?.ipv6) return `${wan.ipv6.split('/')[0]}, no prefix for your networks`;
		return "Your provider isn't offering it right now";
	});
	const prefixSizes = [
		{ value: 'auto', label: 'Automatic' },
		...[48, 56, 60, 64].map((size) => ({ value: String(size), label: `/${size}` }))
	];

	async function saveIpv6() {
		busy = 'ipv6';
		try {
			await applyChange(
				() =>
					api.put<Applied>('/internet/ipv6', {
						enabled: ipv6,
						prefix: prefixSize === 'auto' ? null : Number(prefixSize)
					}),
				ipv6 ? 'IPv6 settings saved' : 'IPv6 is off'
			);
			onsaved();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}

	async function saveQueue() {
		busy = 'queue';
		try {
			await api.put('/internet/qos', {
				enabled: queue,
				mode,
				download: kbit(download),
				upload: kbit(upload)
			});
			toasts.show(
				!queue
					? 'Smart queue is off'
					: settings.acceleration === 'hardware'
						? 'Smart queue is on, acceleration switched to software'
						: mode === 'speedtest'
							? 'Measuring your line first'
							: 'Smart queue is on'
			);
			onsaved();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}

	async function setAcceleration(mode: Acceleration) {
		try {
			await api.put('/internet/acceleration', { mode });
			toasts.show(mode === 'off' ? 'Acceleration off' : 'Acceleration on');
			onsaved();
		} catch (err) {
			toasts.error(err);
		}
	}
</script>

<div class="flex flex-col divide-y divide-line">
	<Setting title="IPv6" hint={ipv6Status}>
		<Toggle bind:checked={ipv6} label="IPv6" />
	</Setting>
	{#if ipv6}
		<Setting title="Prefix size" hint="Ask your provider for this much">
			<div class="w-36">
				<Select bind:value={prefixSize} options={prefixSizes} />
			</div>
		</Setting>
	{/if}
	{#if ipv6Changed}
		<div class="py-3">
			<Button variant="primary" class="w-full" onclick={saveIpv6} busy={busy === 'ipv6'}>
				Save IPv6
			</Button>
		</div>
	{/if}
	<Setting title="Smart queue" hint="Keeps calls smooth during big downloads">
		<Toggle bind:checked={queue} label="Smart queue" />
	</Setting>
	{#if queue}
		<Setting title="Speeds" hint={modeHint[mode]} stacked>
			<Segmented
				bind:value={mode}
				options={[
					{ value: 'adaptive', label: 'Follow the line' },
					{ value: 'speedtest', label: 'Speed test' },
					{ value: 'fixed', label: 'Fixed' }
				]}
			/>
		</Setting>
	{/if}
	{#if queue && mode !== 'speedtest'}
		<div class="grid grid-cols-2 gap-3 py-3">
			<TextField
				bind:value={download}
				label={mode === 'adaptive' ? 'Plan download, Mbps' : 'Download, Mbps'}
				inputmode="decimal"
			/>
			<TextField
				bind:value={upload}
				label={mode === 'adaptive' ? 'Plan upload, Mbps' : 'Upload, Mbps'}
				inputmode="decimal"
			/>
		</div>
	{/if}
	{#if queueChanged}
		<div class="py-3">
			<Button variant="primary" class="w-full" onclick={saveQueue} busy={busy === 'queue'}>
				Save smart queue
			</Button>
		</div>
	{/if}
	<Setting
		title="Acceleration"
		hint={{
			off: 'Every packet goes through the processor',
			software: 'Fast, and everything keeps working',
			hardware: 'Fastest, but skips the smart queue, limits and priority'
		}[settings.acceleration]}
	>
		<div class="w-36">
			<Select
				value={settings.acceleration}
				onchange={setAcceleration}
				options={[
					{ value: 'off', label: 'Off' },
					{ value: 'software', label: 'Software' },
					{ value: 'hardware', label: 'Hardware' }
				]}
			/>
		</div>
	</Setting>
</div>
