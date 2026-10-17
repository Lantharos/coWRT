<script lang="ts">
	type Props = {
		checked: boolean;
		label: string;
		disabled?: boolean;
		onchange?: (checked: boolean) => void;
	};

	let { checked = $bindable(), label, disabled = false, onchange }: Props = $props();

	function toggle() {
		checked = !checked;
		onchange?.(checked);
	}
</script>

<button
	type="button"
	role="switch"
	aria-checked={checked}
	aria-label={label}
	{disabled}
	onclick={toggle}
	class="relative h-6 w-10 shrink-0 cursor-pointer rounded-full transition-colors duration-200 disabled:cursor-default disabled:opacity-50 {checked
		? 'bg-accent'
		: 'bg-overlay'}"
>
	<span
		class="absolute top-0.5 left-0.5 size-5 rounded-full bg-fg shadow-sm transition-transform duration-200 {checked
			? 'translate-x-4'
			: ''}"
	></span>
</button>
