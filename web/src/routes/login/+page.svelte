<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { api, ApiError } from '#lib/api/client.js';
	import type { SetupStatus } from '#lib/api/types.js';
	import Logo from '#lib/components/layout/Logo.svelte';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';

	let password = $state('');
	let error = $state<string | null>(null);
	let busy = $state(false);

	onMount(async () => {
		const setup = await api.get<SetupStatus>('/setup').catch(() => null);
		if (setup?.needed && !setup.passwordSet) goto('/setup');
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		busy = true;
		error = null;
		try {
			await api.post('/auth/login', { password });
			await goto('/');
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'The router is not responding';
			password = '';
		} finally {
			busy = false;
		}
	}
</script>

<div class="grid min-h-dvh place-items-center px-4">
	<form onsubmit={submit} class="flex w-full max-w-[360px] animate-rise flex-col gap-8">
		<div class="flex flex-col items-center gap-5 text-center">
			<Logo size={56} />
			<div>
				<h1 class="font-display text-3xl font-medium">Welcome back</h1>
				<p class="mt-1.5 text-muted">Enter your router password to continue.</p>
			</div>
		</div>
		<div class="flex flex-col gap-3">
			<TextField
				bind:value={password}
				secret
				placeholder="Password"
				autofocus
				aria-label="Password"
			/>
			{#if error}<p class="px-1 text-sm text-danger">{error}</p>{/if}
			<Button variant="primary" type="submit" {busy} disabled={!password} class="h-11"
				>Sign in</Button
			>
		</div>
	</form>
</div>
