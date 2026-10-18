<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	type Props = HTMLButtonAttributes & {
		variant?: 'primary' | 'subtle' | 'ghost' | 'danger';
		size?: 'sm' | 'md';
		busy?: boolean;
		children: Snippet;
	};

	let {
		variant = 'subtle',
		size = 'md',
		busy = false,
		disabled,
		class: className = '',
		children,
		...rest
	}: Props = $props();

	const variants = {
		primary: 'bg-accent text-[#06130a] hover:bg-accent-strong',
		subtle: 'bg-raised text-fg hover:bg-overlay',
		ghost: 'text-muted hover:text-fg hover:bg-raised',
		danger: 'bg-danger-soft text-danger hover:bg-danger hover:text-white'
	};
	const sizes = { sm: 'h-8 px-3 text-[13px] gap-1.5', md: 'h-10 px-4 text-sm gap-2' };
</script>

<button
	{...rest}
	disabled={disabled || busy}
	class="inline-flex shrink-0 cursor-pointer items-center justify-center rounded-control font-medium whitespace-nowrap transition-colors duration-150 select-none disabled:cursor-default disabled:opacity-50 {variants[
		variant
	]} {sizes[size]} {className}"
>
	{#if busy}
		<span class="size-3.5 animate-spin rounded-full border-2 border-current border-t-transparent"
		></span>
	{/if}
	{@render children()}
</button>
