<script lang="ts">
	import { untrack } from 'svelte';
	import { Ban, Power, Unplug } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import type { Device, Person } from '#lib/api/types.js';
	import { bandLabel, bits, bytes, joined, linkRate } from '#lib/format.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import AreaChart from '#lib/components/charts/AreaChart.svelte';
	import Facts from '#lib/components/ui/Facts.svelte';
	import Select from '#lib/components/ui/controls/Select.svelte';
	import Setting from '#lib/components/ui/Setting.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';
	import DeviceControls from './DeviceControls.svelte';
	import Sparkline from '#lib/components/charts/Sparkline.svelte';
	import { kinds, kindOf, type Kind } from './kinds';
	import { recentSignal, signalLabel } from './signal';

	type Props = {
		device: Device | null;
		people: Person[];
		onclose: () => void;
		onchange: () => void;
	};
	let { device, people, onclose, onchange }: Props = $props();

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
	const signal = $derived(device ? recentSignal(device.signals) : null);

	async function wake() {
		if (!device) return;
		saving = 'wake';
		try {
			await api.post(`/devices/${device.mac}/wake`);
			toasts.show(`Woke ${device.name}`);
		} catch (err) {
			toasts.error(err);
		} finally {
			saving = null;
		}
	}

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
				{device.online ? 'Online' : 'Offline'}{#if device.isYou}<span class="text-muted">
						· you're using this</span
					>{/if}
			</p>
		{/if}
	{/snippet}

	{#if device}
		<div class="flex flex-col gap-8">
			<div class="flex flex-col gap-3">
				<div class="flex items-baseline justify-between text-[14px]">
					<span>
						<span class="font-display text-2xl"
							>{joined(bits(device.usage.down + device.usage.up))}</span
						>
						<span class="text-muted">right now</span>
					</span>
					<span class="text-muted"
						>{joined(bytes(device.usage.todayDown + device.usage.todayUp))} today</span
					>
				</div>
				<AreaChart
					values={device.usage.recent}
					max={Math.max(200_000, ...device.usage.recent) * 1.15}
					height={56}
				/>
				{#if connection?.kind === 'wifi'}
					<div class="flex gap-2">
						<Button size="sm" onclick={() => disconnect(0)} busy={saving === 'disconnect'}>
							<Unplug size={15} /> Disconnect
						</Button>
						<Button
							size="sm"
							variant="ghost"
							onclick={() => disconnect(60)}
							busy={saving === 'block'}
						>
							<Ban size={15} /> Block for an hour
						</Button>
					</div>
				{:else if !device.online && device.ipv4}
					<Button size="sm" class="self-start" onclick={wake} busy={saving === 'wake'}>
						<Power size={15} /> Wake up
					</Button>
				{/if}
			</div>

			{#if connection?.kind === 'wifi' && signal !== null}
				<div class="flex items-center justify-between gap-4">
					<div>
						<p class="text-[15px]">Signal</p>
						<p class="text-[13px] {signalLabel(signal) === 'Weak' ? 'text-warn' : 'text-muted'}">
							{signalLabel(signal)} · {signal} dBm{#if signalLabel(signal) === 'Weak'}, closer to
								the router helps{/if}
						</p>
					</div>
					<Sparkline
						values={device.signals.map((dbm) => dbm + 100)}
						floor={70}
						color={signalLabel(signal) === 'Weak' ? 'var(--color-warn)' : undefined}
					/>
				</div>
			{/if}

			<DeviceControls {device} {onchange} />

			<div class="flex flex-col">
				{#if people.length}
					<Setting title="Belongs to">
						<div class="w-44">
							<Select
								value={device.owner ?? ''}
								options={[
									{ value: '', label: 'Nobody' },
									...people.map((p) => ({ value: p.id, label: p.name }))
								]}
								onchange={(owner) =>
									update(
										'owner',
										{ owner: owner || null },
										owner ? `Belongs to ${people.find((p) => p.id === owner)?.name}` : 'Unassigned'
									)}
							/>
						</div>
					</Setting>
				{/if}
				<Setting title="Type">
					<div class="w-44">
						<Select
							value={kind}
							options={Object.entries(kinds).map(([value, item]) => ({
								value: value as Kind,
								label: item.label
							}))}
							onchange={saveKind}
						/>
					</div>
				</Setting>
				<Setting title="Reserve address" hint="Always hand out the same IP">
					<Toggle bind:checked={reserve} label="Reserve address" />
				</Setting>
				{#if reserve}
					<TextField
						bind:value={reservedIp}
						placeholder="10.0.0.50"
						aria-label="Reserved address"
					/>
				{/if}
				{#if reserve !== (device.reservedIp !== null) || (reserve && reservedIp !== device.reservedIp)}
					<Button
						variant="primary"
						class="mt-3"
						onclick={saveReservation}
						busy={saving === 'reservation'}
					>
						Save reservation
					</Button>
				{/if}
			</div>

			<Facts
				facts={[
					{ label: 'IP address', value: device.ipv4, copy: true },
					...(device.ipv6.length ? [{ label: 'IPv6', value: device.ipv6[0], copy: true }] : []),
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
		</div>
	{/if}
</Sheet>
