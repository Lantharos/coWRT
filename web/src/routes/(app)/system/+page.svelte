<script lang="ts">
	import {
		Archive,
		ArrowUpCircle,
		Cpu,
		HardDrive,
		HardDriveUpload,
		KeyRound,
		MemoryStick,
		Power,
		Router,
		ScrollText
	} from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { opened } from '#lib/opened.svelte.js';
	import { live } from '#lib/api/live.svelte.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { SystemInfo } from '#lib/api/types.js';
	import { bytes, duration, joined } from '#lib/format.js';
	import BackupSheet from '#lib/components/system/BackupSheet.svelte';
	import FirmwareSheet from '#lib/components/system/FirmwareSheet.svelte';
	import Health, { type Metric } from '#lib/components/system/Health.svelte';
	import Log from '#lib/components/system/Log.svelte';
	import PasswordSheet from '#lib/components/system/PasswordSheet.svelte';
	import RestartSheet from '#lib/components/system/RestartSheet.svelte';
	import RouterSheet from '#lib/components/system/RouterSheet.svelte';
	import UpdatesSheet from '#lib/components/system/updates/UpdatesSheet.svelte';
	import List from '#lib/components/ui/List.svelte';
	import PageHeader from '#lib/components/ui/PageHeader.svelte';
	import Row from '#lib/components/ui/Row.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';

	type Panel =
		Metric | 'router' | 'updates' | 'backups' | 'firmware' | 'password' | 'log' | 'restart';

	const system = poll(() => api.get<SystemInfo>('/system'), 60000);

	let panel = $state<Panel | null>(null);

	opened('open', (value) => (panel = value as Panel));

	const close = () => (panel = null);
	const snapshot = $derived(live.snapshot);
	const recent = $derived(live.samples.slice(-60));
	const memory = $derived(snapshot?.memory);
	const storage = $derived(system.data?.storage);
	const storageFull = $derived(storage ? storage.used / Math.max(1, storage.total) > 0.85 : false);
	const tint = 'var(--color-accent)';
</script>

<PageHeader
	detail={snapshot && system.data
		? `${system.data.model}, running for ${duration(snapshot.uptime)}.`
		: undefined}
>
	{#snippet headline()}
		{system.data?.hostname ?? 'System'}
	{/snippet}
</PageHeader>

<div class="flex max-w-[860px] flex-col gap-10">
	<List>
		<Row
			title="Processor"
			detail={system.data?.processor}
			icon={Cpu}
			{tint}
			graph={recent.map((s) => s.cpu)}
			graphMax={100}
			value={snapshot ? `${Math.round(snapshot.cpu)}%` : null}
			onclick={() => (panel = 'cpu')}
		/>
		<Row
			title="Memory"
			detail={memory ? `${joined(bytes(memory.used))} of ${joined(bytes(memory.total))}` : null}
			icon={MemoryStick}
			{tint}
			graph={recent.map((s) => (s.memory.used / s.memory.total) * 100)}
			graphMax={100}
			value={memory ? `${Math.round((memory.used / memory.total) * 100)}%` : null}
			onclick={() => (panel = 'memory')}
		/>
		<Row
			title="Storage"
			detail={storage ? `${joined(bytes(storage.used))} of ${joined(bytes(storage.total))}` : null}
			icon={HardDrive}
			tint={storageFull ? 'var(--color-danger)' : tint}
			value={storage ? `${Math.round((storage.used / Math.max(1, storage.total)) * 100)}%` : null}
			tone={storageFull ? 'bad' : 'muted'}
		/>
	</List>

	<List>
		<Row
			title="This router"
			detail="Name, time zone and versions"
			icon={Router}
			value={system.data?.zonename?.replaceAll('_', ' ')}
			onclick={() => (panel = 'router')}
		/>
		<Row
			title="Updates"
			icon={ArrowUpCircle}
			value={system.data ? `coWRT ${system.data.cowrt}` : null}
			onclick={() => (panel = 'updates')}
		/>
		<Row title="Backups" icon={Archive} onclick={() => (panel = 'backups')} />
		<Row title="Firmware" icon={HardDriveUpload} onclick={() => (panel = 'firmware')} />
		<Row title="Password" icon={KeyRound} onclick={() => (panel = 'password')} />
		<Row title="Log" icon={ScrollText} onclick={() => (panel = 'log')} />
		<Row title="Restart" icon={Power} onclick={() => (panel = 'restart')} />
	</List>
</div>

<Sheet
	open={panel === 'cpu' || panel === 'memory'}
	title={panel === 'cpu' ? 'Processor' : 'Memory'}
	onclose={close}
>
	<Health metric={panel === 'memory' ? 'memory' : 'cpu'} />
</Sheet>
<Sheet open={panel === 'log'} title="Log" onclose={close}>
	<Log />
</Sheet>
<RouterSheet
	system={system.data}
	open={panel === 'router'}
	onclose={close}
	onsaved={() => system.refresh()}
/>
<UpdatesSheet current={system.data?.cowrt ?? ''} open={panel === 'updates'} onclose={close} />
<BackupSheet open={panel === 'backups'} onclose={close} />
<FirmwareSheet open={panel === 'firmware'} onclose={close} />
<PasswordSheet open={panel === 'password'} onclose={close} />
<RestartSheet open={panel === 'restart'} onclose={close} />
