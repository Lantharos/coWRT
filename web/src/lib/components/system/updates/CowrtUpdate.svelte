<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '#lib/api/client.js';
	import type { UpdateInfo } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Toggle from '#lib/components/ui/controls/Toggle.svelte';
	import Setting from '#lib/components/ui/Setting.svelte';

	type Props = { current: string; open: boolean };
	let { current, open }: Props = $props();

	let info = $state<UpdateInfo | null>(null);
	let unreachable = $state(false);
	let busy = $state(false);

	export async function check() {
		try {
			info = await api.get<UpdateInfo>('/system/update');
			unreachable = false;
		} catch {
			unreachable = true;
		}
	}

	$effect(() => {
		if (open) untrack(check);
	});

	const notes = $derived(
		info?.notes
			?.split('\n')
			.filter((line) => !line.startsWith('**Full Changelog**'))
			.map((line) =>
				line
					.replace(/^#+\s*/, '')
					.replace(/^[*-]\s+/, '• ')
					.replaceAll('**', '')
			)
			.join('\n')
			.trim()
	);

	async function setAutomatic(enabled: boolean) {
		if (!info) return;
		info.automatic = enabled;
		try {
			await api.put('/system/update/automatic', { enabled });
			toasts.show(
				enabled ? 'coWRT updates itself between 3 and 5 at night' : 'Updates wait for you'
			);
		} catch (err) {
			info.automatic = !enabled;
			toasts.error(err);
		}
	}

	async function install() {
		busy = true;
		try {
			await api.post('/system/update');
			toasts.show('Installing. The page reconnects by itself in a moment.');
			setTimeout(() => location.reload(), 15000);
		} catch (err) {
			toasts.error(err);
			busy = false;
		}
	}
</script>

<section class="flex flex-col gap-3">
	<h3 class="text-[15px]">coWRT</h3>
	<p class="text-[14px] text-muted">
		{#if !info && unreachable}
			Couldn't check for updates right now.
		{:else if !info}
			Looking for updates…
		{:else if info.available}
			{info.latest} is out. You're on {info.current}.
		{:else}
			You're on the newest version, {info.current || current}.
		{/if}
	</p>
	{#if info?.available}
		{#if notes}
			<p class="text-[14px] whitespace-pre-line text-muted">{notes}</p>
		{/if}
		<Button variant="primary" class="self-start" onclick={install} {busy}>
			Install {info.latest}
		</Button>
	{/if}
	{#if info}
		<Setting
			title="Install updates overnight"
			hint="Only coWRT. OpenWrt updates always wait for you"
		>
			<Toggle checked={info.automatic} label="Install updates overnight" onchange={setAutomatic} />
		</Setting>
	{/if}
</section>
