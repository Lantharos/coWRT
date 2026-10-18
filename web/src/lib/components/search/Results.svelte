<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { goto } from '$app/navigation';
	import { Search } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { devices } from '#lib/api/devices.svelte.js';
	import type { NetworksOverview, Person, WifiOverview } from '#lib/api/types.js';
	import { entries, matches, type Entry, type Sources } from './entries';

	let { onclose }: { onclose: () => void } = $props();

	devices.use();

	let query = $state('');
	let active = $state(0);
	let list = $state<HTMLDivElement>();
	let input: HTMLInputElement;
	let loaded = $state<Omit<Sources, 'devices'>>({ people: [], wifi: null, networks: null });

	onMount(async () => {
		input.focus();
		const [people, wifi, networks] = await Promise.all([
			api.get<Person[]>('/people').catch(() => []),
			api.get<WifiOverview>('/wifi').catch(() => null),
			api.get<NetworksOverview>('/networks').catch(() => null)
		]);
		loaded = { people, wifi, networks };
	});

	const shown = $derived(
		entries({ ...loaded, devices: devices.list ?? [] }).filter((entry) => matches(entry, query))
	);
	const groups = $derived(
		shown.reduce<[string, Entry[]][]>((all, entry) => {
			const group = all.find(([name]) => name === entry.group);
			if (group) group[1].push(entry);
			else all.push([entry.group, [entry]]);
			return all;
		}, [])
	);
	const ordered = $derived(groups.flatMap(([, items]) => items));

	function open(entry: Entry) {
		onclose();
		goto(entry.href);
	}

	async function keydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown') active = Math.min(active + 1, ordered.length - 1);
		else if (event.key === 'ArrowUp') active = Math.max(active - 1, 0);
		else if (event.key === 'Enter' && ordered[active]) open(ordered[active]);
		else return;
		event.preventDefault();
		await tick();
		list?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' });
	}
</script>

<label class="flex items-center gap-3 px-5 py-4 text-faint">
	<Search size={18} class="shrink-0" />
	<input
		bind:value={query}
		oninput={() => (active = 0)}
		onkeydown={keydown}
		placeholder="Search devices, people, settings"
		aria-label="Search"
		bind:this={input}
		class="w-full bg-transparent text-[17px] text-fg outline-none placeholder:text-faint"
	/>
</label>
<div bind:this={list} role="listbox" class="max-h-[min(60vh,480px)] overflow-y-auto px-2 pb-2">
	{#each groups as [group, items] (group)}
		<p class="px-3 pt-3 pb-1 text-[13px] text-faint">{group}</p>
		{#each items as entry (entry.href)}
			{@const index = ordered.indexOf(entry)}
			<div
				role="option"
				tabindex="-1"
				aria-selected={index === active}
				onpointerenter={() => (active = index)}
				onclick={() => open(entry)}
				onkeydown={() => {}}
				class="flex cursor-pointer items-center gap-3 rounded-[12px] px-3 py-2.5 {index === active
					? 'bg-white/6'
					: ''}"
			>
				<entry.icon size={17} />
				<span class="min-w-0 flex-1 truncate text-[15px]">{entry.label}</span>
				{#if entry.detail}<span class="shrink-0 text-[13px] text-faint">{entry.detail}</span>{/if}
			</div>
		{/each}
	{:else}
		<p class="px-3 py-6 text-center text-[14px] text-muted">Nothing matches.</p>
	{/each}
</div>
