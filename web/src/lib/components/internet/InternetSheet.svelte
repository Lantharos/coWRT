<script lang="ts">
	import type { InternetOverview } from '#lib/api/types.js';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import ConnectionForm from './ConnectionForm.svelte';
	import LineSettings from './LineSettings.svelte';

	type Props = {
		settings: InternetOverview | null;
		open: boolean;
		onclose: () => void;
		onsaved: () => void;
	};
	let { settings, open, onclose, onsaved }: Props = $props();
</script>

<Sheet {open} title="Internet" {onclose}>
	{#if settings}
		<div class="flex flex-col gap-8">
			<ConnectionForm {settings} {onsaved} />
			<LineSettings {settings} {onsaved} />
		</div>
	{/if}
</Sheet>
