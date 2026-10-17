import { ApiError } from './api/client';

type Toast = { id: number; message: string; tone: 'info' | 'error' };

class Toasts {
	items = $state<Toast[]>([]);
	#next = 0;

	show(message: string, tone: Toast['tone'] = 'info') {
		const id = this.#next++;
		this.items.push({ id, message, tone });
		setTimeout(() => this.dismiss(id), tone === 'error' ? 6000 : 3200);
	}

	error(err: unknown) {
		this.show(err instanceof ApiError ? err.message : 'The router is not responding', 'error');
	}

	dismiss(id: number) {
		this.items = this.items.filter((toast) => toast.id !== id);
	}
}

export const toasts = new Toasts();
