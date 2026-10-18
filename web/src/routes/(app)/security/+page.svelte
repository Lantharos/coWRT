<script lang="ts">
	import { untrack } from 'svelte';
	import { X } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { poll } from '#lib/api/poll.svelte.js';
	import type { DnsProvider, NetworksOverview, Protection } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import { networkColors } from '#lib/components/networks/colors.js';
	import Block from '#lib/components/ui/Block.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Section from '#lib/components/ui/Section.svelte';
	import Segmented from '#lib/components/ui/Segmented.svelte';
	import Select from '#lib/components/ui/Select.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';

	type Level = 'off' | 'threats' | 'ads';

	const protection = poll(() => api.get<Protection>('/protection'), 60000);
	const networks = poll(() => api.get<NetworksOverview>('/networks'), 60000);

	let domain = $state('');
	let provider = $state<DnsProvider>('off');
	let nextdns = $state('');
	let busy = $state(false);

	const dnsKey = $derived(
		`${protection.data?.privateDns.provider}|${protection.data?.privateDns.nextdns}`
	);

	$effect.pre(() => {
		if (!dnsKey) return;
		untrack(() => {
			provider = protection.data?.privateDns.provider ?? 'off';
			nextdns = protection.data?.privateDns.nextdns ?? '';
		});
	});

	function level(id: string): Level {
		if (protection.data?.ads.includes(id)) return 'ads';
		if (protection.data?.threats.includes(id)) return 'threats';
		return 'off';
	}

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
		const label = {
			off: 'No blocking',
			threats: 'Blocking threats',
			ads: 'Blocking ads and threats'
		};
		save({ ads, threats }, `${label[next]} now`);
	}

	function allow(event: SubmitEvent) {
		event.preventDefault();
		if (!protection.data || !domain.trim()) return;
		save(
			{ allow: [...protection.data.allow, domain.trim()] },
			`${domain.trim()} is always allowed`
		);
		domain = '';
	}

	async function saveDns() {
		busy = true;
		if (provider !== 'off') toasts.show('Setting up encrypted DNS');
		try {
			await api.put('/protection/dns', { provider, nextdns: nextdns.trim() || null });
			toasts.show(provider === 'off' ? 'Using your provider’s DNS' : 'Lookups are encrypted now');
			protection.refresh();
		} catch (err) {
			toasts.error(err);
		} finally {
			busy = false;
		}
	}

	const providers: { value: DnsProvider; label: string; detail?: string }[] = [
		{ value: 'off', label: 'Off', detail: 'Use whatever your internet provider gives you' },
		{ value: 'cloudflare', label: 'Cloudflare', detail: 'Fast, logs are wiped within a day' },
		{ value: 'quad9', label: 'Quad9', detail: 'Swiss, also blocks known malware' },
		{ value: 'mullvad', label: 'Mullvad', detail: 'No logs at all' },
		{ value: 'google', label: 'Google' },
		{ value: 'adguard', label: 'AdGuard', detail: 'Blocks ads at the source' },
		{ value: 'nextdns', label: 'NextDNS', detail: 'Your own filtering profile' }
	];
	const dnsChanged = $derived(
		provider !== protection.data?.privateDns.provider ||
			(provider === 'nextdns' && nextdns !== (protection.data?.privateDns.nextdns ?? ''))
	);
	const updated = $derived(
		protection.data?.updated
			? `Lists refreshed ${new Date(protection.data.updated * 1000).toLocaleString()}.`
			: 'Lists refresh every day.'
	);
</script>

<div>
	<h1 class="font-display text-[42px] leading-none font-medium">Security</h1>
	<p class="mt-2 text-muted">
		Keep ads, trackers and bad sites away from every device on a network.
	</p>
</div>

<Section title="Blocking">
	<Block
		description="Works for every device on the network, no apps needed. Devices that try to use their own DNS are caught too. {updated}"
	/>
	{#each networks.data?.networks ?? [] as network (network.id)}
		<div
			class="flex flex-col gap-3 bg-surface px-6 py-4 sm:flex-row sm:items-center sm:justify-between"
		>
			<span class="flex items-center gap-2.5 text-[17px] font-medium">
				<span class="size-3 rounded-full" style:background={networkColors[network.color]}></span>
				{network.name}
			</span>
			<div class="sm:w-[340px]">
				<Segmented
					value={level(network.id)}
					onchange={(next) => setLevel(network.id, next)}
					options={[
						{ value: 'off', label: 'Off' },
						{ value: 'threats', label: 'Threats' },
						{ value: 'ads', label: 'Ads and threats' }
					]}
				/>
			</div>
		</div>
	{/each}
</Section>

<Section title="Always allow">
	<Block description="Sites that break because of blocking. Their subdomains are allowed too.">
		<div class="flex flex-col gap-3">
			{#each protection.data?.allow ?? [] as entry (entry)}
				<div class="flex items-center justify-between gap-3 rounded-control bg-raised px-3.5 py-2">
					<span class="text-[15px]">{entry}</span>
					<button
						type="button"
						aria-label="Stop allowing {entry}"
						disabled={busy}
						onclick={() =>
							save(
								{ allow: protection.data!.allow.filter((d) => d !== entry) },
								`${entry} is blocked again`
							)}
						class="grid size-7 cursor-pointer place-items-center rounded-full text-faint hover:text-danger"
					>
						<X size={16} />
					</button>
				</div>
			{/each}
			<form class="flex items-end gap-2" onsubmit={allow}>
				<TextField
					bind:value={domain}
					placeholder="example.com"
					class="flex-1"
					aria-label="Domain"
				/>
				<Button type="submit" class="h-11" disabled={!domain.trim()} {busy}>Allow</Button>
			</form>
		</div>
	</Block>
</Section>

<Section title="Encrypted DNS">
	<Block
		description="Sends the router's lookups over HTTPS, so your internet provider can't see or change which sites you look up."
	>
		<div class="flex flex-col gap-4">
			<Select bind:value={provider} options={providers} />
			{#if provider === 'nextdns'}
				<TextField bind:value={nextdns} label="NextDNS configuration ID" placeholder="abc123" />
			{/if}
			{#if dnsChanged}
				<Button variant="primary" onclick={saveDns} {busy}>Save</Button>
			{/if}
		</div>
	</Block>
</Section>
