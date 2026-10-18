<script lang="ts" generics="T extends string">
	import { Check } from '@lucide/svelte';

	type Option = { value: T; label: string; detail?: string };
	type Props = { value: T; options: Option[]; label: string; onchange?: (value: T) => void };
	let { value = $bindable(), options, label, onchange }: Props = $props();

	function choose(next: T) {
		value = next;
		onchange?.(next);
	}
</script>

<div role="radiogroup" aria-label={label} class="flex flex-col gap-0.5">
	{#each options as option (option.value)}
		{@const chosen = option.value === value}
		<button
			type="button"
			role="radio"
			aria-checked={chosen}
			onclick={() => choose(option.value)}
			class="flex min-h-14 cursor-pointer items-center justify-between gap-4 rounded-[14px] px-3.5 py-2.5 text-left transition-colors {chosen
				? 'bg-raised'
				: 'hover:bg-raised/60'}"
		>
			<span class="flex min-w-0 flex-col">
				<span class="text-[15px]">{option.label}</span>
				{#if option.detail}<span class="text-[13px] text-muted">{option.detail}</span>{/if}
			</span>
			{#if chosen}<Check size={17} class="shrink-0 text-accent" />{/if}
		</button>
	{/each}
</div>
