<script lang="ts">
	import { untrack } from 'svelte';
	import { Ban, Unplug } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import type { Device } from '#lib/api/types.js';
	import { bandLabel, linkRate } from '#lib/format.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/Button.svelte';
	import Facts from '#lib/components/ui/Facts.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import Toggle from '#lib/components/ui/Toggle.svelte';
	import { kinds, kindOf, type Kind } from './kinds';

	type Props = { device: Device | null; onclose: () => void; onchange: () => void };
	let { device, onclose, onchange }: Props = $props();

	let name = $state('');
	let reserve = $state(false);
	let reservedIp = $state('');
	let saving = $state<string | null>(null);

	const key = $derived(device ? `${device.mac}|${device.customName}|${device.reservedIp}` : null);

	$effect.pre(() => {
		if (!key) return;
		untrack(() => {
			if (!device) return;
			name = device.customName ?? '';
			reserve = device.reservedIp !== null;
			reservedIp = device.reservedIp ?? device.ipv4 ?? '';
		});
	});

	const kind = $derived(device ? kindOf(device) : 'other');
	const connection = $derived(device?.connection);

	async function update(task: string, body: Record<string, unknown>, message: string) {
		if (!device) return;
		saving = task;
		try {
			await api.patch(`/devices/${device.mac}`, body);
			toasts.show(message);
			onchange();
		} catch (err) {
			toasts.error(err);
		} finally {
			saving = null;
		}
	}

	function saveName() {
		if (!device || name.trim() === (device.customName ?? '')) return;
		update('name', { name: name.trim() || null }, name.trim() ? 'Name saved' : 'Name reset');
	}

	function saveKind(next: Kind) {
		update('icon', { icon: next }, 'Device type updated');
	}

	function saveReservation() {
		update(
			'reservation',
			{ reservedIp: reserve ? reservedIp.trim() : null },
			reserve ? `Reserved ${reservedIp.trim()}` : 'Reservation removed'
		);
	}

	async function disconnect(blockMinutes: number) {
		if (!device) return;
		saving = blockMinutes ? 'block' : 'disconnect';
		try {
			await api.post(`/devices/${device.mac}/disconnect`, { blockMinutes });
			toasts.show(blockMinutes ? `Blocked from Wi-Fi for ${blockMinutes} minutes` : 'Disconnected');
			onchange();
		} catch (err) {
			toasts.error(err);
		} finally {
			saving = null;
		}
	}
</script>

<Sheet open={device !== null} title={device?.name ?? ''} {onclose}>
	{#snippet header()}
		{#if device}
			<input
				bind:value={name}
				placeholder={device.hostname ?? device.mac.toUpperCase()}
				onblur={saveName}
				onkeydown={(event) => event.key === 'Enter' && event.currentTarget.blur()}
				aria-label="Device name"
				class="-mx-2 w-full rounded-lg bg-transparent px-2 py-0.5 font-display text-2xl font-medium outline-none placeholder:text-fg hover:bg-raised focus:bg-raised"
			/>
			<p class="mt-1 text-sm {device.online ? 'text-accent' : 'text-faint'}">
				{device.online ? 'Online' : 'Offline'}{#if device.isYou}&nbsp;· this is the device you're
					using{/if}
			</p>
		{/if}
	{/snippet}

	{#if device}
		<div class="flex flex-col gap-8">
			{#if connection?.kind === 'wifi'}
				<div class="grid grid-cols-2 gap-2">
					<Button onclick={() => disconnect(0)} busy={saving === 'disconnect'}>
						<Unplug size={16} /> Disconnect
					</Button>
					<Button variant="danger" onclick={() => disconnect(60)} busy={saving === 'block'}>
						<Ban size={16} /> Block for 1 hour
					</Button>
				</div>
			{/if}

			<Facts
				facts={[
					{ label: 'IP address', value: device.ipv4, copy: true },
					{ label: 'MAC address', value: device.mac.toUpperCase(), copy: true },
					{ label: 'Hostname', value: device.hostname },
					...(connection?.kind === 'wifi'
						? [
								{ label: 'Network', value: `${connection.ssid} · ${bandLabel[connection.band]}` },
								{
									label: 'Signal',
									value: connection.signal === null ? null : `${connection.signal} dBm`
								},
								{
									label: 'Link speed',
									value: `${linkRate(connection.rxRate)} down · ${linkRate(connection.txRate)} up`
								}
							]
						: [
								{
									label: 'Connection',
									value: connection?.kind === 'wired' ? 'Wired' : 'Not connected'
								}
							])
				]}
			/>

			<div class="flex flex-col gap-3">
				<h3 class="text-[15px] font-medium">Type</h3>
				<div class="grid grid-cols-4 gap-1.5">
					{#each Object.entries(kinds) as [key, item] (key)}
						<button
							type="button"
							onclick={() => saveKind(key as Kind)}
							class="flex cursor-pointer flex-col items-center gap-1.5 rounded-control px-1 py-3 text-xs transition-colors {kind ===
							key
								? 'bg-accent-soft text-accent'
								: 'bg-raised text-muted hover:text-fg'}"
						>
							<item.icon size={18} />
							{item.label}
						</button>
					{/each}
				</div>
			</div>

			<div class="flex flex-col gap-4 rounded-[18px] bg-raised/60 p-4">
				<div class="flex items-center justify-between gap-4">
					<div>
						<h3 class="text-[15px] font-medium">Reserve address</h3>
						<p class="text-[13px] text-muted">Always hand this device the same IP.</p>
					</div>
					<Toggle bind:checked={reserve} label="Reserve address" />
				</div>
				{#if reserve}
					<TextField
						bind:value={reservedIp}
						placeholder="192.168.1.50"
						aria-label="Reserved address"
					/>
				{/if}
				{#if reserve !== (device.reservedIp !== null) || (reserve && reservedIp !== device.reservedIp)}
					<Button variant="primary" onclick={saveReservation} busy={saving === 'reservation'}>
						Save reservation
					</Button>
				{/if}
			</div>
		</div>
	{/if}
</Sheet>
