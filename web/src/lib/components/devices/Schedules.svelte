<script lang="ts">
	import { Plus, X } from '@lucide/svelte';
	import type { Day, Schedule } from '#lib/api/types.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';

	type Props = { schedules: Schedule[]; busy: boolean; onsave: (schedules: Schedule[]) => void };
	let { schedules, busy, onsave }: Props = $props();

	const days: { value: Day; short: string }[] = [
		{ value: 'mon', short: 'M' },
		{ value: 'tue', short: 'T' },
		{ value: 'wed', short: 'W' },
		{ value: 'thu', short: 'T' },
		{ value: 'fri', short: 'F' },
		{ value: 'sat', short: 'S' },
		{ value: 'sun', short: 'S' }
	];
	const names: Record<Day, string> = {
		mon: 'Mon',
		tue: 'Tue',
		wed: 'Wed',
		thu: 'Thu',
		fri: 'Fri',
		sat: 'Sat',
		sun: 'Sun'
	};

	let draft = $state<Schedule | null>(null);

	function describe(schedule: Schedule) {
		const set = new Set(schedule.days);
		const label =
			set.size === 7
				? 'Every day'
				: ['mon', 'tue', 'wed', 'thu', 'fri'].every((d) => set.has(d as Day)) && set.size === 5
					? 'Weekdays'
					: ['sun', 'mon', 'tue', 'wed', 'thu'].every((d) => set.has(d as Day)) && set.size === 5
						? 'School nights'
						: set.size === 2 && set.has('sat') && set.has('sun')
							? 'Weekends'
							: days
									.filter((d) => set.has(d.value))
									.map((d) => names[d.value])
									.join(', ');
		return `${label}, ${schedule.from} to ${schedule.to}`;
	}

	function toggleDay(day: Day) {
		if (!draft) return;
		draft.days = draft.days.includes(day)
			? draft.days.filter((d) => d !== day)
			: days.map((d) => d.value).filter((d) => d === day || draft!.days.includes(d));
	}

	function add() {
		if (!draft) return;
		onsave([...schedules, draft]);
		draft = null;
	}
</script>

<div class="flex flex-col gap-2">
	{#each schedules as schedule, index (index)}
		<div class="flex items-center justify-between gap-3 rounded-control bg-raised px-3.5 py-2.5">
			<span class="text-[15px]">{describe(schedule)}</span>
			<button
				type="button"
				aria-label="Remove"
				onclick={() => onsave(schedules.filter((_, i) => i !== index))}
				class="grid size-7 cursor-pointer place-items-center rounded-full text-faint hover:text-danger"
			>
				<X size={16} />
			</button>
		</div>
	{/each}

	{#if draft}
		<div class="flex flex-col gap-4 rounded-[16px] bg-raised p-4">
			<div class="flex justify-between gap-1">
				{#each days as day (day.value)}
					<button
						type="button"
						aria-label={names[day.value]}
						aria-pressed={draft.days.includes(day.value)}
						onclick={() => toggleDay(day.value)}
						class="grid size-9 cursor-pointer place-items-center rounded-full text-sm transition-colors {draft.days.includes(
							day.value
						)
							? 'bg-fg font-medium text-bg'
							: 'bg-overlay text-muted hover:text-fg'}"
					>
						{day.short}
					</button>
				{/each}
			</div>
			<div class="grid grid-cols-2 gap-3">
				<TextField bind:value={draft.from} label="From" placeholder="22:00" maxlength={5} />
				<TextField bind:value={draft.to} label="Until" placeholder="07:00" maxlength={5} />
			</div>
			<div class="flex justify-end gap-2">
				<Button size="sm" variant="ghost" onclick={() => (draft = null)}>Cancel</Button>
				<Button size="sm" variant="primary" {busy} disabled={!draft.days.length} onclick={add}
					>Add</Button
				>
			</div>
		</div>
	{:else}
		<Button
			size="sm"
			variant="ghost"
			class="self-start"
			onclick={() =>
				(draft = { days: ['sun', 'mon', 'tue', 'wed', 'thu'], from: '22:00', to: '07:00' })}
		>
			<Plus size={15} /> Add a time
		</Button>
	{/if}
</div>
