<script lang="ts" generics="T extends string">
	type Props = {
		options: { value: T; label: string }[];
		label?: string;
	} & (
		| { multiple?: false; value: T; onchange?: (value: T) => void }
		| { multiple: true; value: T[]; onchange?: never }
	);

	let { options, label, multiple = false, value = $bindable(), onchange }: Props = $props();

	function isOn(option: T) {
		return multiple ? (value as T[]).includes(option) : value === option;
	}

	function pick(option: T) {
		if (!multiple) {
			value = option;
			onchange?.(option);
			return;
		}
		const current = value as T[];
		value = current.includes(option)
			? current.filter((item) => item !== option)
			: options.map((o) => o.value).filter((item) => item === option || current.includes(item));
	}
</script>

<div class="flex flex-col gap-1.5">
	{#if label}<span class="text-[13px] text-muted">{label}</span>{/if}
	<div class="flex gap-1 rounded-control bg-raised p-1" role={multiple ? 'group' : 'radiogroup'}>
		{#each options as option (option.value)}
			<button
				type="button"
				role={multiple ? 'checkbox' : 'radio'}
				aria-checked={isOn(option.value)}
				onclick={() => pick(option.value)}
				class="h-9 flex-1 cursor-pointer rounded-[9px] px-3 text-sm whitespace-nowrap transition-colors {isOn(
					option.value
				)
					? 'bg-fg font-medium text-bg'
					: 'text-muted hover:text-fg'}"
			>
				{option.label}
			</button>
		{/each}
	</div>
</div>
