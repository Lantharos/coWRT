<script lang="ts">
	import { devices } from '#lib/api/devices.svelte.js';
	import type { Device, Person } from '#lib/api/types.js';
	import { bits, joined } from '#lib/format.js';
	import { palette } from '#lib/palette.js';
	import Row from '#lib/components/ui/Row.svelte';
	import Avatar from './Avatar.svelte';

	type Props = { person: Person; owned: Device[]; onselect: () => void };
	let { person, owned, onselect }: Props = $props();

	const online = $derived(owned.filter((device) => device.online));
	const rate = $derived(online.reduce((sum, d) => sum + d.usage.down + d.usage.up, 0));
	const detail = $derived.by(() => {
		if (!owned.length) return 'No devices yet';
		const count = `${owned.length} ${owned.length === 1 ? 'device' : 'devices'}`;
		return online.length ? `${count} · ${online.length} online` : count;
	});
</script>

<Row
	title={person.name}
	{detail}
	tint={palette[person.color]}
	graph={online.length ? devices.traffic(online.map((d) => d.mac)) : undefined}
	value={person.blocked ? 'Paused' : online.length ? joined(bits(rate)) : null}
	tone={person.blocked ? 'bad' : 'muted'}
	onclick={onselect}
>
	{#snippet leading()}<Avatar {person} />{/snippet}
</Row>
