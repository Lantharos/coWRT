<script lang="ts">
	import { onMount } from 'svelte';
	import { live } from '#lib/api/live.svelte.js';
	import Nav from '#lib/components/layout/Nav.svelte';
	import Toasts from '#lib/components/layout/Toasts.svelte';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();

	onMount(() => {
		live.start();
		return () => live.stop();
	});
</script>

<div
	class="mx-auto flex w-full max-w-[1120px] flex-col gap-10 px-4 pt-[max(1.25rem,env(safe-area-inset-top))] pb-24 sm:px-8 sm:pt-8"
>
	<Nav />
	<main class="flex flex-col gap-12">
		{@render children()}
	</main>
</div>

<Toasts />
