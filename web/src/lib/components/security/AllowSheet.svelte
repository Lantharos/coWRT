<script lang="ts">
	import { X } from '@lucide/svelte';
	import Button from '#lib/components/ui/controls/Button.svelte';
	import Sheet from '#lib/components/ui/Sheet.svelte';
	import TextField from '#lib/components/ui/controls/TextField.svelte';

	type Props = {
		allow: string[];
		open: boolean;
		busy: boolean;
		onclose: () => void;
		onsave: (allow: string[], message: string) => void;
	};
	let { allow, open, busy, onclose, onsave }: Props = $props();

	let domain = $state('');

	function add(event: SubmitEvent) {
		event.preventDefault();
		const entry = domain.trim();
		if (!entry) return;
		onsave([...allow, entry], `${entry} is always allowed`);
		domain = '';
	}
</script>

<Sheet {open} title="Always allow" {onclose}>
	<div class="flex flex-col gap-5">
		<p class="text-[14px] text-muted">For sites that break because of blocking.</p>
		<form class="flex items-end gap-2" onsubmit={add}>
			<TextField bind:value={domain} placeholder="example.com" class="flex-1" aria-label="Domain" />
			<Button type="submit" class="h-11" disabled={!domain.trim()} {busy}>Allow</Button>
		</form>
		<div class="flex flex-col">
			{#each allow as entry (entry)}
				<div class="flex min-h-11 items-center justify-between gap-3">
					<span class="truncate text-[15px]">{entry}</span>
					<button
						type="button"
						aria-label="Stop allowing {entry}"
						disabled={busy}
						onclick={() =>
							onsave(
								allow.filter((d) => d !== entry),
								`${entry} is blocked again`
							)}
						class="grid size-8 cursor-pointer place-items-center rounded-full text-faint hover:bg-raised hover:text-danger"
					>
						<X size={16} />
					</button>
				</div>
			{/each}
		</div>
	</div>
</Sheet>
