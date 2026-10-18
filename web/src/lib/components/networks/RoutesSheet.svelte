<script lang="ts">
	import { api } from '#lib/api/client.js';
	import type { Network, StaticRoute } from '#lib/api/types.js';
	import { toasts } from '#lib/toast.svelte.js';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import EntryList from './EntryList.svelte';

	type Props = {
		routes: StaticRoute[];
		networks: Network[];
		open: boolean;
		onclose: () => void;
		onsaved: () => void;
	};
	let { routes, networks, open, onclose, onsaved }: Props = $props();

	let target = $state('');
	let gateway = $state('');
	let busy = $state(false);

	const networkName = (id: string) => networks.find((n) => n.id === id)?.name ?? id;

	async function change(task: () => Promise<unknown>, message: string) {
		busy = true;
		try {
			await task();
			toasts.show(message);
			onsaved();
			return true;
		} catch (err) {
			toasts.error(err);
			return false;
		} finally {
			busy = false;
		}
	}

	async function add(event: SubmitEvent) {
		event.preventDefault();
		const added = await change(
			() => api.post('/lan/routes', { target: target.trim(), gateway: gateway.trim() }),
			`Traffic for ${target.trim()} goes through ${gateway.trim()}`
		);
		if (added) target = gateway = '';
	}
</script>

<Sheet {open} title="Routes" {onclose}>
	<div class="flex flex-col gap-6">
		<p class="text-[14px] text-muted">Send traffic for other networks through another router.</p>
		<form class="flex flex-col gap-3" onsubmit={add}>
			<div class="grid grid-cols-2 gap-3">
				<TextField bind:value={target} label="Destination" placeholder="192.168.50.0/24" />
				<TextField bind:value={gateway} label="Through" placeholder="10.0.0.2" />
			</div>
			<Button type="submit" variant="primary" {busy} disabled={!target.trim() || !gateway.trim()}>
				Add route
			</Button>
		</form>
		<EntryList
			items={routes}
			empty="No routes yet."
			label={(route) => route.target}
			{busy}
			onremove={(route) =>
				change(() => api.delete(`/lan/routes/${route.id}`), `Route to ${route.target} removed`)}
		>
			{#snippet children(route)}
				<p class="truncate">{route.target}</p>
				<p class="truncate text-[13px] text-muted">
					Through {route.gateway} on {networkName(route.network)}
				</p>
			{/snippet}
		</EntryList>
	</div>
</Sheet>
