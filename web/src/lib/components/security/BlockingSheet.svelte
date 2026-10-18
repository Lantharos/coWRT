<script lang="ts">
	import type { Network } from '#lib/api/types.js';
	import Choices from '#lib/components/ui/controls/Choices.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import type { Level } from './levels';

	type Props = {
		network: Network | null;
		level: Level;
		onclose: () => void;
		onchange: (level: Level) => void;
	};
	let { network, level, onclose, onchange }: Props = $props();
</script>

<Sheet open={!!network} title={network?.name ?? ''} {onclose}>
	<Choices
		value={level}
		label="Blocking on {network?.name}"
		onchange={(next) => {
			onchange(next);
			onclose();
		}}
		options={[
			{ value: 'off', label: 'Off', detail: 'Nothing is filtered' },
			{ value: 'threats', label: 'Threats', detail: 'Malware, phishing and scam sites' },
			{ value: 'ads', label: 'Ads and threats', detail: 'Also ads and trackers' }
		]}
	/>
</Sheet>
