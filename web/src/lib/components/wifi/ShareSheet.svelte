<script lang="ts">
	import { renderSVG } from 'uqr';
	import type { WifiNetwork } from '#lib/api/types.js';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import { joinCode } from './passwords';

	let { network, onclose }: { network: WifiNetwork | null; onclose: () => void } = $props();

	const qr = $derived(
		network
			? renderSVG(joinCode(network.ssid, network.password, network.hidden), {
					border: 2,
					pixelSize: 8
				})
			: null
	);
</script>

<Sheet open={network !== null} title={network?.ssid ?? ''} {onclose}>
	{#if network && qr}
		<div class="flex flex-col items-center gap-6 pt-2">
			<div
				class="w-full max-w-[280px] overflow-hidden rounded-[20px] bg-white p-2 [&_svg]:h-auto [&_svg]:w-full"
			>
				{@html qr}
			</div>
			<p class="text-center text-[14px] text-muted">Point a phone camera at this to join.</p>
			<dl class="flex w-full flex-col">
				<div class="flex justify-between gap-6 py-2 text-[15px]">
					<dt class="text-muted">Name</dt>
					<dd class="truncate">{network.ssid}</dd>
				</div>
				{#if network.password}
					<div class="flex justify-between gap-6 py-2 text-[15px]">
						<dt class="text-muted">Password</dt>
						<dd class="truncate font-mono">{network.password}</dd>
					</div>
				{/if}
			</dl>
		</div>
	{/if}
</Sheet>
