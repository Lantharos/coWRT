<script lang="ts">
	import { untrack } from 'svelte';
	import type { Controls, Person, Priority, Schedule } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Segmented from '#lib/components/ui/controls/Segmented.svelte';
	import Setting from '#lib/components/ui/Setting.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';
	import Schedules from './Schedules.svelte';

	type Props = {
		id: string;
		controls: Controls;
		person?: string;
		owner?: Person;
		onsave: (change: Partial<Controls>) => Promise<unknown>;
	};
	let { id, controls, person, owner, onsave }: Props = $props();

	const priorityLabel: Record<Priority, string> = { low: 'Low', normal: 'Normal', high: 'High' };
	const internetHint = $derived.by(() => {
		if (controls.blocked)
			return person ? 'Paused on all their devices' : 'Paused, home still works';
		if (owner?.blocked) return `Paused along with ${owner.name}`;
		return undefined;
	});
	const priorityHint = $derived.by(() => {
		if (person) return 'For their devices when the connection is busy';
		if (owner && priority === 'normal' && owner.priority !== 'normal') {
			return `${priorityLabel[owner.priority]} like the rest of ${owner.name}'s devices`;
		}
		return 'When the connection is busy';
	});
	const limitHint = $derived.by(() => {
		if (limited) return person ? 'In Mbps, shared by all their devices' : 'In Mbps';
		if (owner && (owner.limitDown !== null || owner.limitUp !== null)) {
			return `Shares ${owner.name}'s limit`;
		}
		return undefined;
	});

	let busy = $state<string | null>(null);
	let limited = $state(false);
	let down = $state('');
	let up = $state('');
	let priority = $state<Priority>('normal');

	const key = $derived(`${id}|${controls.limitDown}|${controls.limitUp}|${controls.priority}`);

	$effect.pre(() => {
		if (!key) return;
		untrack(() => {
			limited = controls.limitDown !== null || controls.limitUp !== null;
			down = controls.limitDown ? String(controls.limitDown / 1000) : '';
			up = controls.limitUp ? String(controls.limitUp / 1000) : '';
			priority = controls.priority;
		});
	});

	async function save(task: string, change: Partial<Controls>, message: string) {
		busy = task;
		try {
			await onsave(change);
			toasts.show(message);
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}

	const kbit = (mbps: string) => (mbps.trim() ? Math.round(Number(mbps) * 1000) : null);

	function saveLimits() {
		const change = limited
			? { limitDown: kbit(down), limitUp: kbit(up) }
			: { limitDown: null, limitUp: null };
		save('limits', change, limited ? 'Speed limit saved' : 'Speed limit removed');
	}

	function setPriority(next: Priority) {
		priority = next;
		const message = {
			low: 'Gives way to everything else',
			normal: 'Back to normal',
			high: 'Goes first now'
		};
		save('priority', { priority: next }, message[next]);
	}

	function setInternet(on: boolean) {
		const message = person
			? on
				? `${person} is back online`
				: `Paused ${person}`
			: on
				? 'Internet is back on'
				: 'Internet paused';
		save('blocked', { blocked: !on }, message);
	}

	const limitsChanged = $derived(
		limited !== (controls.limitDown !== null || controls.limitUp !== null) ||
			(limited && (kbit(down) !== controls.limitDown || kbit(up) !== controls.limitUp))
	);
</script>

<div class="flex flex-col">
	<Setting title="Internet" hint={internetHint}>
		<Toggle
			checked={!controls.blocked}
			label="Internet access"
			disabled={busy === 'blocked'}
			onchange={setInternet}
		/>
	</Setting>

	<Setting title="Priority" hint={priorityHint}>
		<div class="w-56">
			<Segmented
				value={priority}
				options={(['low', 'normal', 'high'] as const).map((value) => ({
					value,
					label: priorityLabel[value]
				}))}
				onchange={setPriority}
			/>
		</div>
	</Setting>

	<Setting title="Speed limit" hint={limitHint}>
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

	<Setting
		title={person ? 'Bedtime' : 'Quiet times'}
		hint={person
			? 'No internet on their devices during these hours'
			: 'No internet during these hours'}
		stacked
	>
		<Schedules
			schedules={controls.schedules}
			busy={busy === 'schedules'}
			onsave={(schedules: Schedule[]) =>
				save('schedules', { schedules }, person ? 'Bedtime saved' : 'Quiet times saved')}
		/>
	</Setting>
</div>
