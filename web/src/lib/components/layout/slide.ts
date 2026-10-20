import type { onNavigate } from '$app/navigation';
import { tabIndex } from './tabs';

type Navigation = Parameters<Parameters<typeof onNavigate>[0]>[0];

export function slideBetweenTabs(navigation: Navigation) {
	if (!document.startViewTransition || matchMedia('(prefers-reduced-motion: reduce)').matches)
		return;
	const from = tabIndex(navigation.from?.url.pathname ?? '');
	const to = tabIndex(navigation.to?.url.pathname ?? '');
	if (from < 0 || to < 0 || from === to) return;
	document.documentElement.dataset.slide = to > from ? 'forward' : 'back';
	return new Promise<void>((resolve) => {
		document.startViewTransition(async () => {
			resolve();
			await navigation.complete;
		});
	});
}
