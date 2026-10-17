<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { Cpu, Gauge, Globe, LogOut, MonitorSmartphone, Wifi } from '@lucide/svelte';
	import { api } from '#lib/api/client.js';
	import { live } from '#lib/api/live.svelte.js';
	import Logo from './Logo.svelte';

	const tabs = [
		{ href: '/', label: 'Overview', icon: Gauge },
		{ href: '/devices', label: 'Devices', icon: MonitorSmartphone },
		{ href: '/wifi', label: 'Wi-Fi', icon: Wifi },
		{ href: '/network', label: 'Network', icon: Globe },
		{ href: '/system', label: 'System', icon: Cpu }
	];

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
		class="fixed inset-x-0 bottom-0 z-30 flex justify-around bg-bg/90 px-2 pt-1.5 pb-[max(0.375rem,env(safe-area-inset-bottom))] backdrop-blur-lg sm:static sm:z-auto sm:flex-1 sm:justify-start sm:gap-1 sm:bg-transparent sm:p-0 sm:backdrop-blur-none"
	>
		{#each tabs as tab (tab.href)}
			<a
				href={tab.href}
				aria-current={active(tab.href) ? 'page' : undefined}
				class="flex flex-col items-center gap-1 rounded-[13px] px-3 py-1.5 text-[11px] whitespace-nowrap transition-colors sm:flex-row sm:px-3.5 sm:py-2 sm:text-[15px] {active(
					tab.href
				)
					? 'text-fg sm:bg-raised'
					: 'text-faint hover:text-fg sm:text-muted'}"
			>
				<tab.icon size={20} class="sm:hidden" />
				{tab.label}
			</a>
		{/each}
	</nav>
	<button
		type="button"
		onclick={signOut}
		aria-label="Sign out"
		title="Sign out"
		class="grid size-9 shrink-0 cursor-pointer place-items-center rounded-full text-faint transition-colors hover:bg-raised hover:text-fg"
	>
		<LogOut size={17} />
	</button>
</header>
