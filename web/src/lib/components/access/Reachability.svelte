<script lang="ts">
	import { api } from '#lib/api/client.js';
	import type { Reach } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Block from '#lib/components/ui/Block.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Facts from '#lib/components/ui/Facts.svelte';

	let { reach }: { reach: Reach } = $props();

	let busy = $state(false);
	let result = $state<boolean | null>(null);

	async function test() {
		busy = true;
		try {
			result = (await api.post<{ open: boolean }>('/forwards/check', {})).open;
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}

	const verdict = $derived(
		result === null
			? reach.behindNat
				? "There's another router or your provider's network between you and the internet. Port forwards probably won't be reachable from outside."
				: 'Your router has its own public address, so port forwards should work.'
			: result
				? 'The internet reached your router. Port forwards will work.'
				: 'Nothing from outside got through. Port forwards won’t work until that changes.'
	);
</script>

<Block title="Reachability" description={verdict}>
	{#snippet aside()}
		<Button size="sm" onclick={test} {busy}>Test now</Button>
	{/snippet}
	<Facts
		facts={[
			{ label: 'Public address', value: reach.publicIp, copy: true },
			{
				label: 'Router address',
				value: reach.wanIp,
				tone: reach.behindNat ? 'bad' : 'good'
			}
		]}
	/>
</Block>
