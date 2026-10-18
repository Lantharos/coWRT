<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';
	import { Eye, EyeOff } from '@lucide/svelte';

	type Props = Omit<HTMLInputAttributes, 'value'> & {
		value: string;
		label?: string;
		hint?: string;
		secret?: boolean;
		mono?: boolean;
	};

	let {
		value = $bindable(),
		label,
		hint,
		secret = false,
		mono = false,
		class: className = '',
		...rest
	}: Props = $props();

	let revealed = $state(false);
	const id = $props.id();
</script>

<div class="flex flex-col gap-1.5 {className}">
	{#if label}
		<label for={id} class="text-[13px] text-muted">{label}</label>
	{/if}
	<div
		class="relative rounded-control bg-raised field-focus transition-[background-color] hover:bg-overlay has-[:focus]:bg-overlay"
	>
		<input
			{id}
			{...rest}
			bind:value
			type={secret && !revealed ? 'password' : (rest.type ?? 'text')}
			spellcheck="false"
			autocomplete="off"
			class="h-11 w-full rounded-control bg-transparent px-3.5 text-[15px] text-fg outline-none placeholder:text-faint {secret
				? 'pr-11'
				: ''} {mono ? 'tracking-tight' : ''}"
		/>
		{#if secret}
			<button
				type="button"
				onclick={() => (revealed = !revealed)}
				aria-label={revealed ? 'Hide' : 'Show'}
				class="absolute top-1/2 right-2 grid size-8 -translate-y-1/2 cursor-pointer place-items-center rounded-lg text-faint outline-none hover:text-fg focus-visible:text-fg"
			>
				{#if revealed}<EyeOff size={17} />{:else}<Eye size={17} />{/if}
			</button>
		{/if}
	</div>
	{#if hint}
		<p class="text-[13px] text-faint">{hint}</p>
	{/if}
</div>
