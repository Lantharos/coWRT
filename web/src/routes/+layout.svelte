<script lang="ts">
	import './layout.css';
	import logo from '#lib/assets/logo.svg';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
</script>

<svelte:head>
	<link rel="icon" href={logo} />
	<title>coWRT</title>
</svelte:head>

{@render children()}
