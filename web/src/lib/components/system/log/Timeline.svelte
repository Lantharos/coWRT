<script lang="ts">
	import { api } from '#lib/api/client.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { TimelineEvent } from '#lib/api/types.js';

	const events = poll(() => api.get<TimelineEvent[]>('/system/events'), 15000);

	const dot = {
		info: 'bg-faint',
		good: 'bg-accent',
		warn: 'bg-warn',
		bad: 'bg-danger'
	};

	function day(seconds: number) {
		const date = new Date(seconds * 1000);
		const today = new Date();
		const yesterday = new Date(today.getTime() - 86400000);
		if (date.toDateString() === today.toDateString()) return 'Today';
		if (date.toDateString() === yesterday.toDateString()) return 'Yesterday';
		return date.toLocaleDateString([], { weekday: 'long', day: 'numeric', month: 'long' });
	}

	const clock = (seconds: number) =>
		new Date(seconds * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

	const days = $derived.by(() => {
		const groups: { day: string; events: TimelineEvent[] }[] = [];
		for (const event of events.data ?? []) {
			const label = day(event.time);
			if (groups.at(-1)?.day !== label) groups.push({ day: label, events: [] });
			groups.at(-1)?.events.push(event);
		}
		return groups;
	});
</script>

<div class="flex flex-col gap-6">
	{#each days as group (group.day)}
		<section class="flex flex-col gap-1">
			<h3 class="pb-1 text-[13px] text-muted">{group.day}</h3>
			{#each group.events as event, index (index)}
				<div class="flex items-baseline gap-3 py-1 text-[15px]">
					<span class="w-[4.5rem] shrink-0 text-[13px] whitespace-nowrap text-faint tabular-nums"
						>{clock(event.time)}</span
					>
					<span class="size-2 shrink-0 translate-y-[-1px] rounded-full {dot[event.tone]}"></span>
					<span>{event.message}</span>
				</div>
			{/each}
		</section>
	{:else}
		<p class="text-[14px] text-muted">
			{events.data ? 'Nothing has happened yet.' : 'Loading…'}
		</p>
	{/each}
</div>
