<script lang="ts">
	import type { Snippet } from 'svelte';
	import { ChevronRight } from '@lucide/svelte';

	type Props = {
		title: string;
		detail?: string | null;
		value?: string | null;
		tone?: 'muted' | 'good' | 'bad';
		dimmed?: boolean;
		leading?: Snippet;
		trailing?: Snippet;
		onclick?: () => void;
		href?: string;
	};
	let {
		title,
		detail,
		value,
		tone = 'muted',
		dimmed = false,
		leading,
		trailing,
		onclick,
		href
	}: Props = $props();

	const interactive = $derived(Boolean(onclick || href));
	const toneClass = { muted: 'text-muted', good: 'text-accent', bad: 'text-danger' };
</script>

{#snippet body()}
	{#if leading}<span class="flex shrink-0 items-center">{@render leading()}</span>{/if}
	<span class="flex min-w-0 flex-1 flex-col">
		<span class="truncate text-[16px] {dimmed ? 'text-muted' : 'text-fg'}">{title}</span>
		{#if detail}<span class="truncate text-[14px] text-muted">{detail}</span>{/if}
	</span>
	{#if value}
		<span class="max-w-[45%] shrink-0 truncate text-right text-[15px] {toneClass[tone]}"
			>{value}</span
		>
	{/if}
{/snippet}

{#snippet content()}
	{@render body()}
	{#if trailing}<span class="flex shrink-0 items-center">{@render trailing()}</span>{/if}
	{#if interactive && !trailing}
		<ChevronRight
			size={18}
			class="shrink-0 text-faint transition-transform group-hover:translate-x-0.5"
		/>
	{/if}
{/snippet}

{#if href}
	<a
		{href}
		class="group flex min-h-16 items-center gap-4 rounded-[16px] px-3 py-3 transition-colors hover:bg-surface"
	>
		{@render content()}
	</a>
{:else if onclick && !trailing}
	<button
		type="button"
		{onclick}
		class="group flex min-h-16 w-full cursor-pointer items-center gap-4 rounded-[16px] px-3 py-3 text-left transition-colors hover:bg-surface"
	>
		{@render content()}
	</button>
{:else if onclick}
	<div
		class="group flex min-h-16 items-center gap-4 rounded-[16px] pr-3 transition-colors hover:bg-surface"
	>
		<button
			type="button"
			{onclick}
			class="flex min-w-0 flex-1 cursor-pointer items-center gap-4 py-3 pl-3 text-left"
		>
			{@render body()}
		</button>
		{#if trailing}{@render trailing()}{/if}
	</div>
{:else}
	<div class="flex min-h-16 items-center gap-4 rounded-[16px] px-3 py-3">
		{@render content()}
	</div>
{/if}
