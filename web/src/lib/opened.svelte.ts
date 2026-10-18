import { untrack } from 'svelte';
import { replaceState } from '$app/navigation';
import { page } from '$app/state';

export function opened(name: string, apply: (value: string) => void) {
	$effect(() => {
		const value = page.url.searchParams.get(name);
		if (!value) return;
		untrack(() => apply(value));
		const url = new URL(page.url.href);
		url.searchParams.delete(name);
		replaceState(url, page.state);
	});
}
