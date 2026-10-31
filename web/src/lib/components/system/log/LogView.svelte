<script lang="ts">
	import Segmented from '#lib/components/ui/controls/Segmented.svelte';
	import SystemLog from './SystemLog.svelte';
	import Timeline from './Timeline.svelte';

	let view = $state<'timeline' | 'system'>('timeline');
</script>

<div class="flex flex-col gap-6">
	<Segmented
		bind:value={view}
		options={[
			{ value: 'timeline', label: 'What happened' },
			{ value: 'system', label: 'System log' }
		]}
	/>
	{#if view === 'timeline'}
		<Timeline />
	{:else}
		<SystemLog />
	{/if}
</div>
