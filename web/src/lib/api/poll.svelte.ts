import { toasts } from '#lib/toast.svelte.js';

class Poll<T> {
	data = $state.raw<T | null>(null);

	constructor(
		private load: () => Promise<T>,
		every: number
	) {
		$effect(() => {
			this.refresh();
			const timer = setInterval(
				() => document.visibilityState === 'visible' && this.refresh(),
				every
			);
			return () => clearInterval(timer);
		});
	}

	async refresh() {
		try {
			this.data = await this.load();
		} catch (err) {
			if (this.data === null) toasts.error(err);
		}
	}
}

export function poll<T>(load: () => Promise<T>, every: number) {
	return new Poll(load, every);
}
