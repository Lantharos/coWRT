import type { Snapshot } from './types';

const WINDOW = 120;

class Live {
	snapshot = $state<Snapshot | null>(null);
	samples = $state.raw<Snapshot[]>([]);
	connected = $state(false);

	#socket: WebSocket | null = null;
	#retry = 0;
	#timer: ReturnType<typeof setTimeout> | undefined;

	start() {
		if (this.#socket) return;
		const url = new URL('/api/live', location.href);
		url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
		const socket = new WebSocket(url);
		this.#socket = socket;
		socket.onopen = () => {
			this.connected = true;
			this.#retry = 0;
		};
		socket.onmessage = (event) => {
			const snapshot: Snapshot = JSON.parse(event.data);
			this.snapshot = snapshot;
			const next = this.samples.length >= WINDOW ? this.samples.slice(1) : this.samples.slice();
			next.push(snapshot);
			this.samples = next;
		};
		socket.onclose = () => {
			this.connected = false;
			this.#socket = null;
			this.#timer = setTimeout(() => this.start(), Math.min(1000 * 2 ** this.#retry++, 15000));
		};
	}

	stop() {
		clearTimeout(this.#timer);
		const socket = this.#socket;
		this.#socket = null;
		if (socket) {
			socket.onclose = null;
			socket.close();
		}
		this.connected = false;
	}
}

export const live = new Live();
