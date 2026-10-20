<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import {
		Cpu,
		Gauge,
		Globe,
		LogOut,
		MonitorSmartphone,
		Network,
		Search,
		Shield,
		Wifi
	} from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { live } from '#lib/api/live.svelte.js';
	import QuickSearch from '#lib/components/search/QuickSearch.svelte';
	import Logo from './Logo.svelte';

	let searching = $state(false);

	const tabs = [
		{ href: '/', label: 'Overview', icon: Gauge },
		{ href: '/devices', label: 'Devices', icon: MonitorSmartphone },
		{ href: '/wifi', label: 'Wi-Fi', icon: Wifi },
		{ href: '/networks', label: 'Networks', icon: Network },
		{ href: '/access', label: 'Access', icon: Globe },
		{ href: '/security', label: 'Security', icon: Shield },
		{ href: '/system', label: 'System', icon: Cpu }
	];

	const updates = $derived(live.snapshot?.updates);
	const hasUpdate = (href: string) => href === '/system' && !!(updates?.cowrt || updates?.openwrt);

	const active = (href: string) =>
		href === '/' ? page.url.pathname === '/' : page.url.pathname.startsWith(href);

	async function signOut() {
		live.stop();
		await api.post('/auth/logout');
		await goto('/login');
	}
</script>

<header class="flex items-center justify-between gap-6">
	<a href="/" class="flex shrink-0 items-center gap-2.5" aria-label="coWRT">
		<Logo size={30} />
		<span class="font-display text-xl font-medium tracking-tight">coWRT</span>
	</a>
	<nav
		class="fixed inset-x-0 bottom-0 z-30 flex justify-around bg-bg/90 px-1 pt-1.5 pb-[max(0.375rem,env(safe-area-inset-bottom))] backdrop-blur-lg md:static md:z-auto md:flex-1 md:justify-start md:gap-1 md:bg-transparent md:p-0 md:backdrop-blur-none"
	>
		{#each tabs as tab (tab.href)}
			<a
				href={tab.href}
				aria-current={active(tab.href) ? 'page' : undefined}
				class="flex min-w-0 flex-1 flex-col items-center gap-1 rounded-[13px] px-0.5 py-1.5 text-[10.5px] whitespace-nowrap transition-colors md:flex-none md:flex-row md:px-3.5 md:py-2 md:text-[15px] {active(
					tab.href
				)
					? 'text-fg md:bg-raised'
					: 'text-faint hover:text-fg md:text-muted'}"
			>
				<span class="relative md:hidden">
					<tab.icon size={20} />
					{#if hasUpdate(tab.href)}
						<span class="absolute -top-0.5 -right-1 size-2 rounded-full bg-accent"></span>
					{/if}
				</span>
				<span class="flex items-center gap-1.5">
					{tab.label}
					{#if hasUpdate(tab.href)}
						<span
							class="hidden size-1.5 rounded-full bg-accent md:block"
							aria-label="Update available"
						></span>
					{/if}
				</span>
			</a>
		{/each}
	</nav>
	<div class="flex shrink-0 items-center gap-1">
		<button
			type="button"
			onclick={() => (searching = true)}
			aria-label="Search"
			title="Search (⌘K)"
			class="grid size-9 shrink-0 cursor-pointer place-items-center rounded-full text-faint transition-colors hover:bg-raised hover:text-fg"
		>
			<Search size={17} />
		</button>
		<button
			type="button"
			onclick={signOut}
			aria-label="Sign out"
			title="Sign out"
			class="grid size-9 shrink-0 cursor-pointer place-items-center rounded-full text-faint transition-colors hover:bg-raised hover:text-fg"
		>
			<LogOut size={17} />
		</button>
	</div>
</header>

<QuickSearch bind:open={searching} />
