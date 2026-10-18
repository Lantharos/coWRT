import { toasts } from '#lib/toast.svelte.js';
import { api } from './client';
import type { SpeedOverview } from './types';

const RUNNING_EVERY = 500;
const IDLE_EVERY = 30000;

class SpeedTests {
	data = $state.raw<SpeedOverview | null>(null);

	#users = 0;
	#timer: ReturnType<typeof setTimeout> | undefined;

	get latest() {
		return this.data?.history.at(-1) ?? null;
	}

	use() {
		$effect(() => {
			if (this.#users++ === 0) this.#tick();
			return () => {
				if (--this.#users === 0) clearTimeout(this.#timer);
			};
		});
		return this;
	}

	async #tick() {
		clearTimeout(this.#timer);
		await this.refresh();
		if (this.#users > 0) {
			this.#timer = setTimeout(() => this.#tick(), this.data?.running ? RUNNING_EVERY : IDLE_EVERY);
		}
	}

	async refresh() {
		const wasRunning = !!this.data?.running;
		const next = await api.get<SpeedOverview>('/internet/speedtest').catch(() => null);
		if (!next) return;
		this.data = next;
		if (wasRunning && !next.running && next.error) toasts.show(next.error, 'error');
	}

	async start() {
		try {
			await api.post('/internet/speedtest');
		} catch (err) {
			toasts.error(err);
		}
		this.#tick();
	}
}

export const speedtests = new SpeedTests();
