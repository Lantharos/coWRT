<script lang="ts">
	import { Check, Copy } from '@lucide/svelte';

	type Fact = {
		label: string;
		value: string | null | undefined;
		copy?: boolean;
		tone?: 'good' | 'bad';
	};
	let { facts }: { facts: Fact[] } = $props();

	let copied = $state<string | null>(null);

	async function copy(value: string) {
		await navigator.clipboard.writeText(value);
		copied = value;
		setTimeout(() => copied === value && (copied = null), 1400);
	}
</script>

<dl class="grid grid-cols-[minmax(110px,auto)_1fr] gap-x-8 gap-y-2.5 text-[15px]">
	{#each facts as fact (fact.label)}
		<dt class="text-muted">{fact.label}</dt>
		<dd
			class="flex min-w-0 items-center gap-2 {fact.tone === 'good'
				? 'text-accent'
				: fact.tone === 'bad'
					? 'text-danger'
					: ''}"
		>
			<span class="truncate">{fact.value || '—'}</span>
			{#if fact.copy && fact.value}
				<button
					type="button"
					onclick={() => copy(fact.value!)}
					aria-label="Copy {fact.label}"
					class="cursor-pointer text-faint transition-colors hover:text-accent"
				>
					{#if copied === fact.value}<Check size={15} class="text-accent" />{:else}<Copy
							size={15}
						/>{/if}
				</button>
			{/if}
		</dd>
	{/each}
</dl>
