<script lang="ts" generics="T extends string">
	import { Check, ChevronDown } from '@lucide/svelte';

	type Option = { value: T; label: string; detail?: string };
	type Props = {
		value: T;
		options: Option[];
		label?: string;
		onchange?: (value: T) => void;
	};

	let { value = $bindable(), options, label, onchange }: Props = $props();

	let open = $state(false);
	let active = $state(0);
	let root: HTMLDivElement;
	const id = $props.id();

	const selected = $derived(options.find((option) => option.value === value));

	function show() {
		active = Math.max(
			0,
			options.findIndex((option) => option.value === value)
		);
		open = true;
	}

	function choose(option: Option) {
		value = option.value;
		open = false;
		onchange?.(option.value);
	}

	function keydown(event: KeyboardEvent) {
		if (!open) {
			if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(event.key)) {
				event.preventDefault();
				show();
			}
			return;
		}
		if (event.key === 'Escape') open = false;
		else if (event.key === 'ArrowDown') active = (active + 1) % options.length;
		else if (event.key === 'ArrowUp') active = (active - 1 + options.length) % options.length;
		else if (event.key === 'Enter' || event.key === ' ') choose(options[active]);
		else return;
		event.preventDefault();
	}
</script>

<svelte:window
	onpointerdown={(event) => open && !root.contains(event.target as Node) && (open = false)}
/>

<div class="flex flex-col gap-1.5" bind:this={root}>
	{#if label}
		<span id="{id}-label" class="text-[13px] text-muted">{label}</span>
	{/if}
	<div class="relative">
		<button
			type="button"
			aria-haspopup="listbox"
			aria-expanded={open}
			aria-labelledby={label ? `${id}-label` : undefined}
			onclick={() => (open ? (open = false) : show())}
			onkeydown={keydown}
			class="flex h-11 w-full cursor-pointer items-center justify-between gap-3 rounded-control bg-raised px-3.5 text-left text-[15px] transition-colors hover:bg-overlay"
		>
			<span class="truncate">{selected?.label ?? value}</span>
			<ChevronDown
				size={16}
				class="shrink-0 text-faint transition-transform {open ? 'rotate-180' : ''}"
			/>
		</button>
		{#if open}
			<ul
				role="listbox"
				class="absolute top-full right-0 left-0 z-30 mt-1.5 max-h-72 animate-rise overflow-y-auto rounded-[16px] bg-overlay p-1.5 shadow-2xl shadow-black/50"
			>
				{#each options as option, index (option.value)}
					<li
						role="option"
						aria-selected={option.value === value}
						onpointerenter={() => (active = index)}
						onclick={() => choose(option)}
						onkeydown={() => {}}
						class="flex cursor-pointer items-center justify-between gap-3 rounded-[10px] px-3 py-2.5 text-[15px] {index ===
						active
							? 'bg-white/6'
							: ''}"
					>
						<span class="flex flex-col">
							<span>{option.label}</span>
							{#if option.detail}<span class="text-[13px] text-faint">{option.detail}</span>{/if}
						</span>
						{#if option.value === value}<Check size={16} class="text-accent" />{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</div>
