<script lang="ts">
	import { Search } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { LogEntry } from '#lib/api/types.js';
	import Segmented from '#lib/components/ui/Segmented.svelte';

	type Filter = 'all' | 'problems';

	const logs = poll(() => api.get<LogEntry[]>('/system/logs?lines=500'), 10000);

	let query = $state('');
	let filter = $state<Filter>('all');

	const entries = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		return (logs.data ?? [])
			.filter((entry) => filter === 'all' || entry.level === 'error' || entry.level === 'warning')
			.filter((entry) => !needle || entry.message.toLowerCase().includes(needle))
			.reverse();
	});

	const tone = { error: 'text-danger', warning: 'text-warn', info: 'text-fg', debug: 'text-muted' };
	const time = (seconds: number) =>
		new Date(seconds * 1000).toLocaleTimeString([], {
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit'
		});
</script>

<div class="flex flex-col gap-4 bg-surface px-6 py-5">
	<div class="flex flex-col gap-3 sm:flex-row">
		<label
			class="flex h-11 flex-1 items-center gap-2.5 rounded-control bg-raised px-3.5 text-muted focus-within:bg-overlay"
		>
			<Search size={17} class="shrink-0" />
			<input
				bind:value={query}
				placeholder="Search the log"
				class="w-full bg-transparent text-[15px] text-fg outline-none placeholder:text-faint"
			/>
		</label>
		<div class="sm:w-56">
			<Segmented
				bind:value={filter}
				options={[
					{ value: 'all', label: 'Everything' },
					{ value: 'problems', label: 'Problems' }
				]}
			/>
		</div>
	</div>
	<div class="max-h-[420px] overflow-y-auto font-mono text-[12px] leading-relaxed">
		{#each entries as entry, index (index)}
			<div class="flex gap-3 py-0.5">
				<span class="shrink-0 text-faint">{time(entry.time)}</span>
				<span class="break-all {tone[entry.level]}">{entry.message}</span>
			</div>
		{:else}
			<p class="py-2 font-sans text-sm text-muted">{logs.data ? 'Nothing matches.' : 'Loading…'}</p>
		{/each}
	</div>
</div>
