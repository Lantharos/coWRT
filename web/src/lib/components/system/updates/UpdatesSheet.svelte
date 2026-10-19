<script lang="ts">
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import CowrtUpdate from './CowrtUpdate.svelte';
	import OpenWrtUpdate from './OpenWrtUpdate.svelte';

	type Props = { current: string; open: boolean; onclose: () => void };
	let { current, open, onclose }: Props = $props();

	let cowrt = $state<CowrtUpdate>();
	let openwrt = $state<OpenWrtUpdate>();
	let checking = $state(false);

	async function check() {
		checking = true;
		await Promise.all([cowrt?.check(), openwrt?.check()]);
		checking = false;
	}
</script>

<Sheet {open} title="Updates" {onclose}>
	<div class="flex flex-col gap-8">
		<CowrtUpdate bind:this={cowrt} {current} {open} />
		<OpenWrtUpdate bind:this={openwrt} {open} />
	</div>

	{#snippet footer()}
		<Button onclick={check} busy={checking}>Check again</Button>
	{/snippet}
</Sheet>
