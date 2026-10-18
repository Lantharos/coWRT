<script lang="ts">
	import { api } from '#lib/api/client.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { LogEntry } from '#lib/api/types.js';
	import SearchField from '#lib/components/ui/controls/SearchField.svelte';
	import Segmented from '#lib/components/ui/controls/Segmented.svelte';

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

<div class="flex flex-col gap-3">
	<SearchField bind:value={query} placeholder="Search the log" />
	<div>
		<Segmented
			bind:value={filter}
			options={[
				{ value: 'all', label: 'Everything' },
				{ value: 'problems', label: 'Problems' }
			]}
		/>
	</div>
</div>
<div class="mt-4 font-mono text-[12px] leading-relaxed">
	{#each entries as entry, index (index)}
		<div class="flex gap-3 py-0.5">
			<span class="shrink-0 text-faint">{time(entry.time)}</span>
			<span class="break-all {tone[entry.level]}">{entry.message}</span>
		</div>
	{:else}
		<p class="py-2 font-sans text-sm text-muted">{logs.data ? 'Nothing matches.' : 'Loading…'}</p>
	{/each}
</div>
