<script lang="ts">
	import { untrack } from 'svelte';
	import { Trash2 } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import type { Color, Device, Person, Schedule } from '#lib/api/types.js';
	import { bytes, joined } from '#lib/format.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Schedules from '#lib/components/devices/Schedules.svelte';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import ColorPicker from '#lib/components/ui/controls/ColorPicker.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';
	import Setting from '#lib/components/ui/Setting.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import Avatar from './Avatar.svelte';

	type Props = {
		person: Person | 'new' | null;
		devices: Device[];
		onclose: () => void;
		onsaved: (id: string) => void;
		ondevices: () => void;
	};
	let { person, devices, onclose, onsaved, ondevices }: Props = $props();

	let name = $state('');
	let color = $state<Color>('blue');
	let busy = $state<string | null>(null);
	let confirmDelete = $state(false);

	const existing = $derived(person && person !== 'new' ? person : null);
	const identity = $derived(person === 'new' ? 'new' : (person?.id ?? null));

	$effect.pre(() => {
		if (!identity) return;
		untrack(() => {
			name = existing?.name ?? '';
			color = existing?.color ?? 'blue';
			confirmDelete = false;
		});
	});

	const owned = $derived(existing ? devices.filter((d) => d.owner === existing.id) : []);
	const choices = $derived(
		existing ? devices.filter((d) => d.owner === null || d.owner === existing.id) : []
	);
	const today = $derived(
		joined(bytes(owned.reduce((sum, d) => sum + d.usage.todayDown + d.usage.todayUp, 0)))
	);
	const renamed = $derived(
		!!existing && (name.trim() !== existing.name || color !== existing.color)
	);

	async function put(change: Partial<Person>, message: string, task: string) {
		if (!existing) return;
		busy = task;
		try {
			const { blocked, schedules } = { ...existing, ...change };
			await api.put(`/people/${existing.id}`, { name: name.trim(), color, blocked, schedules });
			toasts.show(message);
			onsaved(existing.id);
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}

	async function create() {
		busy = 'create';
		try {
			const id = await api.post<string>('/people', {
				name: name.trim(),
				color,
				blocked: false,
				schedules: []
			});
			toasts.show(`${name.trim()} added`);
			onsaved(id);
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}

	async function assign(device: Device, mine: boolean) {
		try {
			await api.patch(`/devices/${device.mac}`, { owner: mine ? existing?.id : null });
			ondevices();
		} catch (err) {
			toasts.error(err);
		}
	}

	async function remove() {
		if (!existing) return;
		if (!confirmDelete) {
			confirmDelete = true;
			return;
		}
		busy = 'delete';
		try {
			await api.delete(`/people/${existing.id}`);
			toasts.show(`${existing.name} removed`);
			onsaved(existing.id);
			ondevices();
			onclose();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = null;
		}
	}
</script>

<Sheet open={person !== null} title={existing?.name ?? 'New person'} {onclose}>
	{#snippet header()}
		{#if existing}
			<div class="flex items-center gap-3.5">
				<Avatar person={{ name, color }} size={48} />
				<div class="min-w-0">
					<h2 class="truncate font-display text-2xl">{existing.name}</h2>
					<p class="text-sm text-muted">{today} today</p>
				</div>
			</div>
		{:else}
			<h2 class="font-display text-2xl">New person</h2>
		{/if}
	{/snippet}

	<div class="flex flex-col gap-8">
		{#if existing}
			<div class="flex flex-col">
				<Setting
					title="Internet"
					hint={existing.blocked ? 'Paused on all their devices' : undefined}
				>
					<Toggle
						checked={!existing.blocked}
						label="Internet"
						disabled={busy === 'blocked'}
						onchange={(on) =>
							put(
								{ blocked: !on },
								on ? `${existing.name} is back online` : `Paused ${existing.name}`,
								'blocked'
							)}
					/>
				</Setting>
				<Setting title="Bedtime" hint="No internet on their devices during these hours" stacked>
					<Schedules
						schedules={existing.schedules}
						busy={busy === 'schedules'}
						onsave={(schedules: Schedule[]) => put({ schedules }, 'Bedtime saved', 'schedules')}
					/>
				</Setting>
			</div>

			<div class="flex flex-col">
				<h3 class="pb-1 text-[15px]">Devices</h3>
				{#each choices as device (device.mac)}
					<Setting title={device.name} hint={device.online ? 'Online' : undefined}>
						<Toggle
							checked={device.owner === existing.id}
							label="{device.name} belongs to {existing.name}"
							onchange={(mine) => assign(device, mine)}
						/>
					</Setting>
				{:else}
					<p class="py-2 text-[14px] text-muted">Every device belongs to someone else.</p>
				{/each}
			</div>
		{/if}

		<div class="flex flex-col gap-5">
			<TextField bind:value={name} label="Name" placeholder="Alex" maxlength={32} />
			<ColorPicker bind:value={color} />
		</div>
	</div>

	{#snippet footer()}
		{#if existing}
			<Button
				variant={confirmDelete ? 'danger' : 'ghost'}
				onclick={remove}
				busy={busy === 'delete'}
				class="mr-auto"
			>
				<Trash2 size={16} />
				{confirmDelete ? 'Really remove?' : 'Remove'}
			</Button>
			{#if renamed}
				<Button
					variant="primary"
					onclick={() => put({}, 'Saved', 'rename')}
					busy={busy === 'rename'}
				>
					Save
				</Button>
			{/if}
		{:else}
			<Button onclick={onclose}>Cancel</Button>
			<Button variant="primary" onclick={create} busy={busy === 'create'} disabled={!name.trim()}>
				Add
			</Button>
		{/if}
	{/snippet}
</Sheet>
