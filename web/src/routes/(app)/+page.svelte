<script lang="ts">
	import { ArrowDown, ArrowUp } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { live } from '#lib/api/live.svelte.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { Device } from '#lib/api/types.js';
	import { bits, bytes } from '#lib/format.js';
	import GridFloor from '#lib/components/charts/GridFloor.svelte';
	import Stat from '#lib/components/overview/Stat.svelte';
	import Status from '#lib/components/overview/Status.svelte';

	const devices = poll(() => api.get<Device[]>('/devices'), 5000);

	const snapshot = $derived(live.snapshot);
	const online = $derived(devices.data?.filter((device) => device.online).length);
	const used = $derived(bytes(snapshot?.usageToday ?? 0));
	const down = $derived(bits(snapshot?.throughput.rx ?? 0));
	const up = $derived(bits(snapshot?.throughput.tx ?? 0));
	const levels = $derived(
		live.samples.map((sample) => sample.throughput.rx + sample.throughput.tx)
	);
</script>

<div class="relative z-10 flex flex-col gap-12 pt-6 sm:pt-14">
	<Status {snapshot} />
	<div class="flex flex-wrap gap-x-14 gap-y-8">
		<Stat value={online === undefined ? '–' : String(online)} label="devices online" />
		<Stat value={used.value} unit={used.unit} label="used today" />
		<Stat value={down.value} unit={down.unit} label="download right now">
			{#snippet lead()}<ArrowDown class="size-7 text-accent" />{/snippet}
		</Stat>
		<Stat value={up.value} unit={up.unit} label="upload right now">
			{#snippet lead()}<ArrowUp class="size-7 text-muted" />{/snippet}
		</Stat>
	</div>
</div>

<div class="pointer-events-none fixed inset-x-0 bottom-0 h-[42dvh] max-h-[460px]">
	<GridFloor {levels} />
</div>
