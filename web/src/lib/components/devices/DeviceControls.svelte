<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import type { Device, Priority, Schedule } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Segmented from '#lib/components/ui/controls/Segmented.svelte';
	import Setting from '#lib/components/ui/Setting.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';
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

<div class="flex flex-col">
	<Setting title="Internet" hint={device.controls.blocked ? 'Paused, home still works' : undefined}>
		<Toggle
			checked={!device.controls.blocked}
			label="Internet access"
			disabled={busy === 'blocked'}
			onchange={(on) =>
				patch('blocked', { blocked: !on }, on ? 'Internet is back on' : 'Internet paused')}
		/>
	</Setting>

	<Setting title="Priority" hint="When the connection is busy">
		<div class="w-56">
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
	</Setting>

	<Setting title="Speed limit" hint={limited ? 'In Mbps' : undefined}>
		<Toggle bind:checked={limited} label="Speed limit" />
	</Setting>
	{#if limited}
		<div class="grid grid-cols-2 gap-3 pb-3">
			<TextField bind:value={down} label="Download" placeholder="No limit" inputmode="decimal" />
			<TextField bind:value={up} label="Upload" placeholder="No limit" inputmode="decimal" />
		</div>
	{/if}
	{#if limitsChanged}
		<Button variant="primary" onclick={saveLimits} busy={busy === 'limits'}>Save speed limit</Button
		>
	{/if}

	<Setting title="Quiet times" hint="No internet during these hours" stacked>
		<Schedules
			schedules={device.controls.schedules}
			busy={busy === 'schedules'}
			onsave={(schedules: Schedule[]) => patch('schedules', { schedules }, 'Quiet times saved')}
		/>
	</Setting>
</div>
