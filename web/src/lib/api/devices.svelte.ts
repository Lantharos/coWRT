import { api } from './client';
import type { Device } from './types';

const EVERY = 4000;

class Devices {
	list = $state.raw<Device[] | null>(null);

	#users = 0;
	#timer: ReturnType<typeof setInterval> | undefined;

	use() {
		$effect(() => {
			if (this.#users++ === 0) {
				this.refresh();
				this.#timer = setInterval(
					() => document.visibilityState === 'visible' && this.refresh(),
					EVERY
				);
			}
			return () => {
				if (--this.#users === 0) clearInterval(this.#timer);
			};
		});
		return this;
	}

	async refresh() {
		const list = await api.get<Device[]>('/devices').catch(() => null);
		if (list) this.list = list;
	}

	traffic(macs: string[]) {
		const wanted = new Set(macs);
		const series = (this.list ?? [])
			.filter((device) => wanted.has(device.mac))
			.map((device) => device.usage.recent);
		const length = Math.max(0, ...series.map((s) => s.length));
		return Array.from({ length }, (_, index) =>
			series.reduce((sum, s) => sum + (s[s.length - length + index] ?? 0), 0)
		);
	}
}

export const devices = new Devices();
