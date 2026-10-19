<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import { applyChange } from '#lib/api/apply.js';
	import type { Applied, InternetConnection, InternetOverview } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Segmented from '#lib/components/ui/controls/Segmented.svelte';
	import Select from '#lib/components/ui/controls/Select.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';

	type Props = { settings: InternetOverview; onsaved: () => void };
	let { settings, onsaved }: Props = $props();

	const initial = untrack(() => settings.connection);
	let kind = $state(initial.kind);
	let port = $state(initial.port);
	let vlan = $state(initial.vlan ? String(initial.vlan) : '');
	let address = $state(initial.address ?? '');
	let gateway = $state(initial.gateway ?? '');
	let dns = $state(initial.dns.join(', '));
	let username = $state(initial.username ?? '');
	let password = $state(initial.password ?? '');
	let mac = $state(initial.mac ?? '');
	let advanced = $state(Boolean(initial.vlan || initial.mac || initial.dns.length));
	let busy = $state(false);

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
		const c = settings.connection;
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

	async function save() {
		busy = true;
		try {
			await applyChange(
				() => api.put<Applied>('/internet/connection', connection),
				'Connection saved'
			);
			onsaved();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

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
		<Button variant="primary" onclick={save} {busy}>Save connection</Button>
	{/if}
</div>
