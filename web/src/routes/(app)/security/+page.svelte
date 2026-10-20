<script lang="ts">
	import { Ban, ListChecks, Lock, Shield, ShieldCheck, ShieldOff } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { opened } from '#lib/opened.svelte.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { NetworksOverview, Protection, ProtectionActivity } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import { palette } from '#lib/palette.js';
	import BlockedSheet from '#lib/components/security/BlockedSheet.svelte';
	import AllowSheet from '#lib/components/security/AllowSheet.svelte';
	import BlockingSheet from '#lib/components/security/BlockingSheet.svelte';
	import DnsSheet from '#lib/components/security/DnsSheet.svelte';
	import { levelLabel, type Level } from '#lib/components/security/levels.js';
	import { timeline } from '#lib/components/security/timeline.js';
	import { providers } from '#lib/components/security/providers.js';
	import List from '#lib/components/ui/List.svelte';
	import PageHeader from '#lib/components/ui/PageHeader.svelte';
	import Row from '#lib/components/ui/Row.svelte';

	const protection = poll(() => api.get<Protection>('/protection'), 60000);
	const networks = poll(() => api.get<NetworksOverview>('/networks'), 60000);
	const activity = poll(() => api.get<ProtectionActivity>('/protection/activity'), 30000);

	let choosing = $state<string | null>(null);
	let panel = $state<'allow' | 'dns' | 'blocked' | null>(null);

	opened('open', (value) => (panel = value as 'allow' | 'dns' | 'blocked'));

	const blockedToday = $derived(
		activity.data?.buckets.reduce((sum, bucket) => sum + bucket.blocked, 0) ?? 0
	);
	let busy = $state(false);

	const all = $derived(networks.data?.networks ?? []);
	const levelIcon = { off: ShieldOff, threats: Shield, ads: ShieldCheck };

	function level(id: string): Level {
		if (protection.data?.ads.includes(id)) return 'ads';
		if (protection.data?.threats.includes(id)) return 'threats';
		return 'off';
	}

	const guarded = $derived(all.filter((network) => level(network.id) !== 'off'));
	const dns = $derived(protection.data?.privateDns);
	const allowed = $derived(protection.data?.allow.length ?? 0);
	const updated = $derived(
		protection.data?.updated
			? `Lists refreshed ${new Date(protection.data.updated * 1000).toLocaleString([], {
					weekday: 'long',
					hour: 'numeric',
					minute: '2-digit'
				})}.`
			: undefined
	);

	async function save(
		change: Partial<Pick<Protection, 'ads' | 'threats' | 'allow'>>,
		message: string
	) {
		if (!protection.data) return;
		busy = true;
		try {
			const { ads, threats, allow } = { ...protection.data, ...change };
			await api.put('/protection', { ads, threats, allow });
			toasts.show(message);
			protection.refresh();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}

	function setLevel(id: string, next: Level) {
		if (!protection.data) return;
		const ads = protection.data.ads.filter((n) => n !== id);
		const threats = protection.data.threats.filter((n) => n !== id);
		if (next === 'ads') ads.push(id);
		if (next === 'threats') threats.push(id);
		const message = {
			off: 'No blocking',
			threats: 'Blocking threats',
			ads: 'Blocking ads and threats'
		};
		save({ ads, threats }, `${message[next]} now`);
	}
</script>

<PageHeader detail={updated}>
	{#snippet headline()}
		{#if !protection.data || !networks.data}
			Security
		{:else if !guarded.length}
			Nothing is <span class="text-danger">blocked</span> yet
		{:else if guarded.length === all.length}
			Every network is <span class="text-accent">protected</span>
		{:else}
			{guarded.length} of {all.length} networks <span class="text-accent">protected</span>
		{/if}
	{/snippet}
</PageHeader>

<div class="flex max-w-[860px] flex-col gap-10">
	<List>
		{#if activity.data}
			<Row
				title="Blocked today"
				detail={activity.data.enabled ? 'Ads, trackers and threats stopped' : 'Not keeping track'}
				icon={Ban}
				tint={activity.data.enabled ? 'var(--color-danger)' : undefined}
				graph={activity.data.enabled
					? timeline(activity.data.buckets, 36, 1, (b) => b.blocked)
					: undefined}
				graphMax={1}
				value={activity.data.enabled ? blockedToday.toLocaleString() : null}
				onclick={() => (panel = 'blocked')}
			/>
		{/if}
	</List>

	<List label="Blocking">
		{#each all as network (network.id)}
			{@const current = level(network.id)}
			<Row
				title={network.name}
				icon={levelIcon[current]}
				tint={current === 'off' ? undefined : palette[network.color]}
				value={levelLabel[current]}
				onclick={() => (choosing = network.id)}
			/>
		{/each}
	</List>

	<List>
		<Row
			title="Always allow"
			detail="Sites that break because of blocking"
			icon={ListChecks}
			value={allowed ? `${allowed} ${allowed === 1 ? 'site' : 'sites'}` : 'None'}
			onclick={() => (panel = 'allow')}
		/>
		<Row
			title="Encrypted DNS"
			detail={protection.data?.dnsFallback
				? "Not answering, so regular DNS is used until it's back"
				: 'Keep lookups private from your provider'}
			icon={Lock}
			tint={protection.data?.dnsFallback
				? 'var(--color-warn)'
				: dns && dns.provider !== 'off'
					? 'var(--color-accent)'
					: undefined}
			value={providers.find((p) => p.value === dns?.provider)?.label ?? 'Off'}
			onclick={() => (panel = 'dns')}
		/>
	</List>
</div>

<BlockingSheet
	network={all.find((n) => n.id === choosing) ?? null}
	level={choosing ? level(choosing) : 'off'}
	onclose={() => (choosing = null)}
	onchange={(next) => choosing && setLevel(choosing, next)}
/>
<BlockedSheet
	activity={activity.data}
	open={panel === 'blocked'}
	onclose={() => (panel = null)}
	onsaved={() => activity.refresh()}
/>
<AllowSheet
	allow={protection.data?.allow ?? []}
	open={panel === 'allow'}
	{busy}
	onclose={() => (panel = null)}
	onsave={(allow, message) => save({ allow }, message)}
/>
<DnsSheet
	current={dns ?? null}
	open={panel === 'dns'}
	onclose={() => (panel = null)}
	onsaved={() => protection.refresh()}
/>
