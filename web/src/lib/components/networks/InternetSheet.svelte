<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import { applyChange } from '#lib/api/apply.js';
	import { live } from '#lib/api/live.svelte.js';
	import type {
		Acceleration,
		Applied,
		ConnectionKind,
		InternetConnection,
		InternetOverview
	} from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Segmented from '#lib/components/ui/controls/Segmented.svelte';
	import Select from '#lib/components/ui/controls/Select.svelte';
	import Setting from '#lib/components/ui/Setting.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';

	type Props = {
		settings: InternetOverview | null;
		open: boolean;
		onclose: () => void;
		onsaved: () => void;
	};
	let { settings, open, onclose, onsaved }: Props = $props();

	let kind = $state<ConnectionKind>('dhcp');
	let port = $state('');
	let vlan = $state('');
	let address = $state('');
	let gateway = $state('');
	let dns = $state('');
	let username = $state('');
	let password = $state('');
	let mac = $state('');
	let queue = $state(false);
	let auto = $state(false);
	let download = $state('');
	let upload = $state('');
	let advanced = $state(false);
	let ipv6 = $state(true);
	let prefixSize = $state('auto');
	let busy = $state<string | null>(null);

	$effect.pre(() => {
		if (!open) return;
		untrack(() => {
			if (!settings) return;
			const c = settings.connection;
			kind = c.kind;
			port = c.port;
			vlan = c.vlan ? String(c.vlan) : '';
			address = c.address ?? '';
			gateway = c.gateway ?? '';
			dns = c.dns.join(', ');
			username = c.username ?? '';
			password = c.password ?? '';
			mac = c.mac ?? '';
			queue = settings.qos.enabled;
			auto = settings.qos.auto;
			download = settings.qos.download ? String(settings.qos.download / 1000) : '';
			upload = settings.qos.upload ? String(settings.qos.upload / 1000) : '';
			advanced = Boolean(c.vlan || c.mac || c.dns.length);
			ipv6 = settings.ipv6.enabled;
			prefixSize = settings.ipv6.prefix ? String(settings.ipv6.prefix) : 'auto';
		});
	});

	const connection = $derived<InternetConnection>({
		kind,
		port,
		vlan: vlan.trim() ? Number(vlan) : null,
		address: kind === 'static' ? address.trim() : null,
		gateway: kind === 'static' ? gateway.trim() : null,
		dns: dns
			.split(/[\s,]+/)
			.map((s) => s.trim())
			.filter(Boolean),
		username: kind === 'pppoe' ? username.trim() : null,
		password: kind === 'pppoe' ? password : null,
		mac: mac.trim() || null
	});

	const connectionChanged = $derived.by(() => {
		const c = settings?.connection;
		if (!c) return false;
		return (
			c.kind !== connection.kind ||
			c.port !== connection.port ||
			c.vlan !== connection.vlan ||
			(c.address ?? null) !== connection.address ||
			(c.gateway ?? null) !== connection.gateway ||
			c.dns.join() !== connection.dns.join() ||
			(c.username ?? null) !== connection.username ||
			(kind === 'pppoe' && (c.password ?? '') !== password) ||
			(c.mac ?? null) !== connection.mac
		);
	});

	const kbit = (mbps: string) => Math.round(Number(mbps || 0) * 1000);
	const queueChanged = $derived(
		!!settings &&
			(queue !== settings.qos.enabled ||
				(queue && auto !== settings.qos.auto) ||
				(queue &&
					!auto &&
					(kbit(download) !== settings.qos.download || kbit(upload) !== settings.qos.upload)))
	);

	async function saveConnection() {
		busy = 'connection';
		try {
			await applyChange(
				() => api.put<Applied>('/internet/connection', connection),
				'Connection saved'
			);
			onsaved();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}

	const ipv6Changed = $derived(
		!!settings &&
			(ipv6 !== settings.ipv6.enabled ||
				(ipv6 && prefixSize !== (settings.ipv6.prefix ? String(settings.ipv6.prefix) : 'auto')))
	);
	const ipv6Status = $derived.by(() => {
		const wan = live.snapshot?.wan;
		if (!ipv6) return undefined;
		if (wan?.prefix6) return `Got ${wan.prefix6}`;
		if (wan?.ipv6) return `${wan.ipv6.split('/')[0]}, no prefix for your networks`;
		return "Your provider isn't offering it right now";
	});
	const prefixSizes = [
		{ value: 'auto', label: 'Automatic' },
		...[48, 56, 60, 64].map((size) => ({ value: String(size), label: `/${size}` }))
	];

	async function saveIpv6() {
		busy = 'ipv6';
		try {
			await applyChange(
				() =>
					api.put<Applied>('/internet/ipv6', {
						enabled: ipv6,
						prefix: prefixSize === 'auto' ? null : Number(prefixSize)
					}),
				ipv6 ? 'IPv6 settings saved' : 'IPv6 is off'
			);
			onsaved();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}

	async function saveQueue() {
		busy = 'queue';
		try {
			await api.put('/internet/qos', {
				enabled: queue,
				auto,
				download: kbit(download),
				upload: kbit(upload)
			});
			toasts.show(
				!queue ? 'Smart queue is off' : auto ? 'Measuring your line first' : 'Smart queue is on'
			);
			onsaved();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}

	async function setAcceleration(mode: Acceleration) {
		try {
			await api.put('/internet/acceleration', { mode });
			toasts.show(mode === 'off' ? 'Acceleration off' : 'Acceleration on');
			onsaved();
		} catch (err) {
			toasts.error(err);
		}
	}
</script>

<Sheet {open} title="Internet" {onclose}>
	{#if settings}
		<div class="flex flex-col gap-8">
			<div class="flex flex-col gap-4">
				<Segmented
					bind:value={kind}
					options={[
						{ value: 'dhcp', label: 'Automatic' },
						{ value: 'static', label: 'Fixed' },
						{ value: 'pppoe', label: 'PPPoE' }
					]}
				/>
				{#if kind === 'static'}
					<TextField bind:value={address} label="Address" placeholder="203.0.113.10/24" />
					<TextField bind:value={gateway} label="Gateway" placeholder="203.0.113.1" />
				{:else if kind === 'pppoe'}
					<TextField bind:value={username} label="Username" />
					<TextField bind:value={password} label="Password" secret />
				{/if}
				{#if advanced}
					<Select
						bind:value={port}
						label="Modem port"
						options={settings.ports.map((p) => ({ value: p, label: p }))}
					/>
					<div class="grid grid-cols-2 gap-3">
						<TextField bind:value={vlan} label="VLAN tag" placeholder="None" inputmode="numeric" />
						<TextField bind:value={mac} label="MAC address" placeholder="Built in" />
					</div>
					<TextField bind:value={dns} label="DNS servers" placeholder="From your provider" />
				{:else}
					<button
						type="button"
						onclick={() => (advanced = true)}
						class="cursor-pointer self-start text-sm text-muted hover:text-fg"
					>
						Port, VLAN tag, MAC and DNS
					</button>
				{/if}
				{#if connectionChanged}
					<Button variant="primary" onclick={saveConnection} busy={busy === 'connection'}>
						Save connection
					</Button>
				{/if}
			</div>

			<div class="flex flex-col divide-y divide-line">
				<Setting title="IPv6" hint={ipv6Status}>
					<Toggle bind:checked={ipv6} label="IPv6" />
				</Setting>
				{#if ipv6}
					<Setting title="Prefix size" hint="Ask your provider for this much">
						<div class="w-36">
							<Select bind:value={prefixSize} options={prefixSizes} />
						</div>
					</Setting>
				{/if}
				{#if ipv6Changed}
					<div class="py-3">
						<Button variant="primary" class="w-full" onclick={saveIpv6} busy={busy === 'ipv6'}>
							Save IPv6
						</Button>
					</div>
				{/if}
				<Setting title="Smart queue" hint="Keeps calls smooth during big downloads">
					<Toggle bind:checked={queue} label="Smart queue" />
				</Setting>
				{#if queue}
					<Setting
						title="Set speeds from the speed test"
						hint={auto && settings.qos.auto && settings.qos.download
							? `${Math.round(settings.qos.download / 1000)} / ${Math.round(settings.qos.upload / 1000)} Mbps`
							: undefined}
					>
						<Toggle bind:checked={auto} label="Set speeds from the speed test" />
					</Setting>
				{/if}
				{#if queue && !auto}
					<div class="grid grid-cols-2 gap-3 py-3">
						<TextField bind:value={download} label="Download, Mbps" inputmode="decimal" />
						<TextField bind:value={upload} label="Upload, Mbps" inputmode="decimal" />
					</div>
				{/if}
				{#if queueChanged}
					<div class="py-3">
						<Button variant="primary" class="w-full" onclick={saveQueue} busy={busy === 'queue'}>
							Save smart queue
						</Button>
					</div>
				{/if}
				<Setting title="Acceleration" hint="Faster, but skips limits and priority">
					<div class="w-36">
						<Select
							value={settings.acceleration}
							onchange={setAcceleration}
							options={[
								{ value: 'off', label: 'Off' },
								{ value: 'software', label: 'Software' },
								{ value: 'hardware', label: 'Hardware' }
							]}
						/>
					</div>
				</Setting>
			</div>
		</div>
	{/if}
</Sheet>
