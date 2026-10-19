<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import type { OpenWrtInfo } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import { waitForRouter } from '../recovery';

	let { open }: { open: boolean } = $props();

	let info = $state<OpenWrtInfo | null>(null);
	let unreachable = $state(false);
	let confirming = $state(false);
	let starting = $state(false);
	let restarting = false;

	export async function check() {
		try {
			info = await api.get<OpenWrtInfo>('/system/openwrt');
			unreachable = false;
		} catch {
			unreachable = true;
		}
	}

	$effect(() => {
		if (open) untrack(check);
	});

	const stage = $derived(info?.progress.stage ?? 'idle');
	const working = $derived(['preparing', 'building', 'verifying'].includes(stage));

	$effect(() => {
		if (!open || !working) return;
		const timer = setInterval(check, 3000);
		return () => clearInterval(timer);
	});

	$effect(() => {
		if (stage !== 'installing' || restarting) return;
		restarting = true;
		waitForRouter();
	});

	async function start() {
		starting = true;
		try {
			await api.post('/system/openwrt');
			confirming = false;
			await check();
		} catch (err) {
			toasts.error(err);
		} finally {
			starting = false;
		}
	}
</script>

<section class="flex flex-col gap-3">
	<h3 class="text-[15px]">OpenWrt</h3>
	<p class="text-[14px] text-muted">
		{#if !info && unreachable}
			Couldn't check for updates right now.
		{:else if !info}
			Looking for updates…
		{:else if stage === 'preparing'}
			Getting ready…
		{:else if stage === 'building'}
			Building OpenWrt {info.latest} with everything you have installed. This usually takes a few minutes,
			and your internet keeps working meanwhile.
		{:else if stage === 'verifying'}
			Checking the new image…
		{:else if stage === 'installing'}
			Installing. The router restarts and is back in a few minutes.
		{:else if info.available}
			{info.latest} is out. You're on {info.current}.
		{:else}
			You're on the newest version, {info.current}.
		{/if}
	</p>
	{#if info?.progress.stage === 'failed'}
		<p class="text-bad text-[14px]">The last update didn't go through: {info.progress.reason}</p>
	{/if}
	{#if info?.available && !working && stage !== 'installing'}
		{#if confirming}
			<div class="flex flex-col gap-4 rounded-[16px] bg-raised p-4">
				<p class="text-[15px]">
					Update to OpenWrt {info.latest}? Your settings, packages and coWRT stay. The router is
					offline for a few minutes while it installs.
				</p>
				<div class="flex justify-end gap-2">
					<Button variant="ghost" onclick={() => (confirming = false)}>Cancel</Button>
					<Button variant="primary" onclick={start} busy={starting}>Update</Button>
				</div>
			</div>
		{:else}
			<Button variant="primary" class="self-start" onclick={() => (confirming = true)}>
				Update to {info.latest}
			</Button>
		{/if}
	{/if}
</section>
