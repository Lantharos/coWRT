<script lang="ts">
	import { untrack } from 'svelte';
	import { Trash2 } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import type { Device, Forward, ForwardInput, ForwardProtocol } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Segmented from '#lib/components/ui/controls/Segmented.svelte';
	import Select from '#lib/components/ui/controls/Select.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';

	type Props = {
		forward: Forward | 'new' | null;
		devices: Device[];
		onclose: () => void;
		onsaved: () => void;
	};
	let { forward, devices, onclose, onsaved }: Props = $props();

	let name = $state('');
	let mac = $state('');
	let protocol = $state<ForwardProtocol>('tcp');
	let external = $state('');
	let internal = $state('');
	let busy = $state<'save' | 'delete' | 'check' | null>(null);
	let checked = $state<boolean | null>(null);

	const key = $derived(forward === 'new' ? 'new' : forward?.id);

	$effect.pre(() => {
		if (!key) return;
		untrack(() => {
			checked = null;
			if (forward === 'new' || !forward) {
				name = '';
				mac = devices.find((d) => d.online)?.mac ?? '';
				protocol = 'tcp';
				external = '';
				internal = '';
				return;
			}
			name = forward.name;
			mac = forward.mac;
			protocol = forward.protocol;
			external = forward.external;
			internal = forward.internal ?? '';
		});
	});

	const options = $derived(
		devices.map((d) => ({
			value: d.mac,
			label: d.name,
			detail: d.reservedIp ?? d.ipv4 ?? undefined
		}))
	);

	async function save() {
		if (!forward) return;
		busy = 'save';
		const body: ForwardInput = {
			name,
			mac,
			protocol,
			external: external.trim(),
			internal: internal.trim() || null,
			enabled: forward === 'new' ? true : forward.enabled
		};
		try {
			if (forward === 'new') await api.post('/forwards', body);
			else await api.put(`/forwards/${forward.id}`, body);
			toasts.show(`${name} is forwarded`);
			onsaved();
			onclose();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}

	async function remove() {
		if (!forward || forward === 'new') return;
		busy = 'delete';
		try {
			await api.delete(`/forwards/${forward.id}`);
			toasts.show(`${forward.name} removed`);
			onsaved();
			onclose();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}

	async function check() {
		if (!forward || forward === 'new') return;
		busy = 'check';
		try {
			checked = (await api.post<{ open: boolean }>('/forwards/check', { id: forward.id })).open;
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}
</script>

<Sheet
	open={forward !== null}
	title={forward === 'new' ? 'New port forward' : (forward?.name ?? '')}
	{onclose}
>
	<div class="flex flex-col gap-6">
		<TextField
			bind:value={name}
			label="Name"
			placeholder="Minecraft server, Plex…"
			maxlength={48}
		/>
		<Select bind:value={mac} {options} label="Send it to" />
		<Segmented
			bind:value={protocol}
			label="Protocol"
			options={[
				{ value: 'tcp', label: 'TCP' },
				{ value: 'udp', label: 'UDP' },
				{ value: 'both', label: 'Both' }
			]}
		/>
		<div class="grid grid-cols-2 gap-3">
			<TextField bind:value={external} label="Outside port" placeholder="25565" />
			<TextField bind:value={internal} label="Device port" placeholder="Same" />
		</div>
		<p class="-mt-3 text-[13px] text-faint">
			Use a range like 8000-8010 to forward several ports. The device keeps its current address so
			the forward doesn't break.
		</p>

		{#if forward !== 'new' && forward}
			<div class="flex items-center justify-between gap-4 rounded-[18px] bg-raised/60 p-4">
				<div>
					<p class="text-[15px]">Check from outside</p>
					<p
						class="text-[13px] {checked === null
							? 'text-muted'
							: checked
								? 'text-accent'
								: 'text-danger'}"
					>
						{checked === null
							? 'Something on the device has to be listening.'
							: checked
								? 'Reachable from the internet.'
								: "Couldn't connect. Is the app running on the device?"}
					</p>
				</div>
				<Button size="sm" onclick={check} busy={busy === 'check'}>Check</Button>
			</div>
		{/if}
	</div>

	{#snippet footer()}
		{#if forward !== 'new'}
			<Button variant="ghost" onclick={remove} busy={busy === 'delete'} class="mr-auto">
				<Trash2 size={16} /> Remove
			</Button>
		{/if}
		<Button onclick={onclose}>Cancel</Button>
		<Button
			variant="primary"
			onclick={save}
			busy={busy === 'save'}
			disabled={!name.trim() || !mac || !external.trim()}
		>
			{forward === 'new' ? 'Create' : 'Save'}
		</Button>
	{/snippet}
</Sheet>
