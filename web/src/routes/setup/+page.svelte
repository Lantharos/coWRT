<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { api } from '#lib/api/client.js';
	import type { SetupStatus } from '#lib/api/types.js';
	import Logo from '#lib/components/layout/Logo.svelte';
	import Toasts from '#lib/components/layout/Toasts.svelte';
	import PasswordStep from '#lib/components/setup/PasswordStep.svelte';
	import TimeStep from '#lib/components/setup/TimeStep.svelte';
	import WifiStep from '#lib/components/setup/WifiStep.svelte';

	type Step = 'password' | 'wifi' | 'time';

	let steps = $state<Step[]>([]);
	let current = $state(0);

	const copy: Record<Step, { title: string; detail: string }> = {
		password: { title: 'Pick a password', detail: 'You use it to sign in here and over SSH.' },
		wifi: {
			title: 'Name your Wi-Fi',
			detail: 'Every band gets the same name, devices pick the best one.'
		},
		time: { title: 'Where is it?', detail: 'Bedtimes and daily totals follow this time zone.' }
	};

	onMount(async () => {
		const [status, session] = await Promise.all([
			api.get<SetupStatus>('/setup'),
			api.get<{ authenticated: boolean }>('/auth/session')
		]);
		if (!status.needed) return goto('/');
		if (status.passwordSet && !session.authenticated) return goto('/login');
		steps = status.passwordSet ? ['wifi', 'time'] : ['password', 'wifi', 'time'];
	});

	function next() {
		if (current < steps.length - 1) current += 1;
		else goto('/');
	}

	const step = $derived(steps[current]);
</script>

<div class="grid min-h-dvh place-items-center px-4 py-10">
	{#if step}
		<div class="flex w-full max-w-[380px] animate-rise flex-col gap-8">
			<div class="flex flex-col items-center gap-5 text-center">
				<Logo size={52} />
				<div>
					<h1 class="font-display text-3xl font-medium">{copy[step].title}</h1>
					<p class="mt-1.5 text-muted">{copy[step].detail}</p>
				</div>
				<div class="flex gap-1.5" aria-hidden="true">
					{#each steps as item, index (item)}
						<span
							class="h-1.5 rounded-full transition-all {index === current
								? 'w-6 bg-accent'
								: index < current
									? 'w-1.5 bg-accent/50'
									: 'w-1.5 bg-overlay'}"
						></span>
					{/each}
				</div>
			</div>
			{#key step}
				{#if step === 'password'}
					<PasswordStep ondone={next} />
				{:else if step === 'wifi'}
					<WifiStep ondone={next} />
				{:else}
					<TimeStep ondone={next} />
				{/if}
			{/key}
		</div>
	{/if}
</div>

<Toasts />
