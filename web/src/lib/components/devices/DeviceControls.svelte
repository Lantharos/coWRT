<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import type { Device, Priority, Schedule } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/Button.svelte';
	import Segmented from '#lib/components/ui/Segmented.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import Toggle from '#lib/components/ui/Toggle.svelte';
	import Schedules from './Schedules.svelte';

	type Props = { device: Device; onchange: () => void };
	let { device, onchange }: Props = $props();

	let busy = $state<string | null>(null);
	let limited = $state(false);
	let down = $state('');
	let up = $state('');
	let priority = $state<Priority>('normal');

	const key = $derived(
		`${device.mac}|${device.controls.limitDown}|${device.controls.limitUp}|${device.controls.priority}`
	);

	$effect.pre(() => {
		if (!key) return;
		untrack(() => {
			const { limitDown, limitUp } = device.controls;
			limited = limitDown !== null || limitUp !== null;
			down = limitDown ? String(limitDown / 1000) : '';
			up = limitUp ? String(limitUp / 1000) : '';
			priority = device.controls.priority;
		});
	});

	async function patch(task: string, body: Record<string, unknown>, message: string) {
		busy = task;
		try {
			await api.patch(`/devices/${device.mac}`, body);
			toasts.show(message);
			onchange();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}

	const kbit = (mbps: string) => (mbps.trim() ? Math.round(Number(mbps) * 1000) : null);

	function saveLimits() {
		const body = limited
			? { limitDown: kbit(down), limitUp: kbit(up) }
			: { limitDown: null, limitUp: null };
		patch('limits', body, limited ? 'Speed limit saved' : 'Speed limit removed');
	}

	function setPriority(next: Priority) {
		priority = next;
		const message = {
			low: 'Gives way to everything else',
			normal: 'Back to normal',
			high: 'Goes first now'
		};
		patch('priority', { priority: next }, message[next]);
	}

	const limitsChanged = $derived(
		limited !== (device.controls.limitDown !== null || device.controls.limitUp !== null) ||
			(limited &&
				(kbit(down) !== device.controls.limitDown || kbit(up) !== device.controls.limitUp))
	);
</script>

<div class="flex flex-col gap-7">
	<div class="flex items-center justify-between gap-6">
		<div>
			<h3 class="text-[15px] font-medium">Internet access</h3>
			<p class="text-[13px] text-muted">
				{device.controls.blocked
					? 'Paused. It can still reach things at home.'
					: 'This device can use the internet.'}
			</p>
		</div>
		<Toggle
			checked={!device.controls.blocked}
			label="Internet access"
			disabled={busy === 'blocked'}
			onchange={(on) =>
				patch('blocked', { blocked: !on }, on ? 'Internet is back on' : 'Internet paused')}
		/>
	</div>

	<div class="flex flex-col gap-3">
		<div>
			<h3 class="text-[15px] font-medium">Quiet times</h3>
			<p class="text-[13px] text-muted">The internet pauses for this device during these hours.</p>
		</div>
		<Schedules
			schedules={device.controls.schedules}
			busy={busy === 'schedules'}
			onsave={(schedules: Schedule[]) => patch('schedules', { schedules }, 'Quiet times saved')}
		/>
	</div>

	<div class="flex flex-col gap-4">
		<div class="flex items-center justify-between gap-6">
			<div>
				<h3 class="text-[15px] font-medium">Speed limit</h3>
				<p class="text-[13px] text-muted">Cap how fast this device can go, in Mbps.</p>
			</div>
			<Toggle bind:checked={limited} label="Speed limit" />
		</div>
		{#if limited}
			<div class="grid grid-cols-2 gap-3">
				<TextField bind:value={down} label="Download" placeholder="No limit" inputmode="decimal" />
				<TextField bind:value={up} label="Upload" placeholder="No limit" inputmode="decimal" />
			</div>
		{/if}
		{#if limitsChanged}
			<Button variant="primary" onclick={saveLimits} busy={busy === 'limits'}
				>Save speed limit</Button
			>
		{/if}
	</div>

	<div class="flex flex-col gap-3">
		<div>
			<h3 class="text-[15px] font-medium">Priority</h3>
			<p class="text-[13px] text-muted">
				Decides who goes first when the connection is busy. Needs the smart queue in Networks.
			</p>
		</div>
		<Segmented
			value={priority}
			options={[
				{ value: 'low', label: 'Low' },
				{ value: 'normal', label: 'Normal' },
				{ value: 'high', label: 'High' }
			]}
			onchange={setPriority}
		/>
	</div>
</div>
