<script lang="ts">
	import { toasts } from '#lib/toast.svelte.js';
</script>

<div
	class="pointer-events-none fixed inset-x-0 bottom-6 z-50 flex flex-col items-center gap-2 px-4"
>
	{#each toasts.items as toast (toast.id)}
		<button
			type="button"
			onclick={() => toasts.dismiss(toast.id)}
			class="pointer-events-auto animate-rise cursor-pointer rounded-full bg-overlay px-5 py-2.5 text-sm shadow-xl shadow-black/40 {toast.tone ===
			'error'
				? 'text-danger'
				: 'text-fg'}"
		>
			{toast.message}
		</button>
	{/each}
</div>
