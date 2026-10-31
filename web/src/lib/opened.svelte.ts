import { afterNavigate, replaceState } from '$app/navigation';
import { page } from '$app/state';

export function opened(name: string, apply: (value: string) => void) {
	afterNavigate(({ to }) => {
		const value = to?.url.searchParams.get(name);
		if (!to || !value) return;
		apply(value);
		const url = new URL(to.url);
		url.searchParams.delete(name);
		replaceState(url, page.state);
	});
}
