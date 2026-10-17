<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import { applyChange } from '#lib/api/apply.js';
	import type { Applied, ConnectionKind, InternetConnection } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/Button.svelte';
	import Segmented from '#lib/components/ui/Segmented.svelte';
	import Select from '#lib/components/ui/Select.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';

	type Props = {
		connection: InternetConnection | null;
		ports: string[];
		open: boolean;
		onclose: () => void;
		onsaved: () => void;
	};
	let { connection, ports, open, onclose, onsaved }: Props = $props();

	let kind = $state<ConnectionKind>('dhcp');
	let port = $state('');
	let vlan = $state('');
	let address = $state('');
	let gateway = $state('');
	let dns = $state('');
	let username = $state('');
	let password = $state('');
	let mac = $state('');
	let busy = $state(false);

	$effect.pre(() => {
		if (!open) return;
		untrack(() => {
			if (!connection) return;
			kind = connection.kind;
			port = connection.port;
			vlan = connection.vlan ? String(connection.vlan) : '';
			address = connection.address ?? '';
			gateway = connection.gateway ?? '';
			dns = connection.dns.join(', ');
			username = connection.username ?? '';
			password = connection.password ?? '';
			mac = connection.mac ?? '';
		});
	});

	async function save() {
		busy = true;
		const body: InternetConnection = {
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
		};
		try {
			const confirmed = await applyChange(
				() => api.put<Applied>('/internet/connection', body),
				'Internet connection saved'
			);
			onsaved();
			if (confirmed) onclose();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}
</script>

<Sheet {open} title="Internet connection" {onclose}>
	<div class="flex flex-col gap-6">
		<Segmented
			bind:value={kind}
			label="How you connect"
			options={[
				{ value: 'dhcp', label: 'Automatic' },
				{ value: 'static', label: 'Fixed address' },
				{ value: 'pppoe', label: 'PPPoE' }
			]}
		/>
		<p class="-mt-3 text-[13px] text-faint">
			{kind === 'dhcp'
				? 'Most cable and fibre modems hand out an address on their own.'
				: kind === 'static'
					? 'For connections where your provider gave you a fixed address.'
					: 'Common for DSL and some fibre. Your provider gives you a username and password.'}
		</p>

		{#if kind === 'static'}
			<TextField bind:value={address} label="Address" placeholder="203.0.113.10/24" />
			<TextField bind:value={gateway} label="Gateway" placeholder="203.0.113.1" />
		{:else if kind === 'pppoe'}
			<TextField bind:value={username} label="Username" />
			<TextField bind:value={password} label="Password" secret />
		{/if}

		<TextField
			bind:value={dns}
			label="DNS servers"
			placeholder="From your provider"
			hint="Leave empty to use your provider's. Separate several with commas."
		/>

		<div class="flex flex-col gap-4 rounded-[18px] bg-raised/60 p-4">
			<Select
				bind:value={port}
				label="Modem plugged into"
				options={ports.map((p) => ({ value: p, label: p }))}
			/>
			<TextField
				bind:value={vlan}
				label="VLAN tag"
				placeholder="None"
				inputmode="numeric"
				hint="Only if your provider tells you to use one."
			/>
			<TextField
				bind:value={mac}
				label="MAC address"
				placeholder="Built in"
				hint="Some providers only talk to the device they first saw."
			/>
		</div>
	</div>

	{#snippet footer()}
		<Button onclick={onclose}>Cancel</Button>
		<Button variant="primary" onclick={save} {busy}>Save</Button>
	{/snippet}
</Sheet>
