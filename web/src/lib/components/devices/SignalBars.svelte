<script lang="ts">
	import { signalQuality } from '#lib/format.js';

	let { dbm }: { dbm: number | null } = $props();
	const quality = $derived(signalQuality(dbm));
</script>

<span class="inline-flex h-3 items-end gap-[2px]" title={dbm === null ? undefined : `${dbm} dBm`}>
	{#each [1, 2, 3, 4] as bar (bar)}
		<span
			class="w-[3px] rounded-full {bar <= quality ? 'bg-current' : 'bg-white/15'}"
			style:height="{bar * 25}%"
		></span>
	{/each}
</span>
