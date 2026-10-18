import { ApiError } from '#lib/api/client.js';
import { toasts } from '#lib/toast.svelte.js';

const pause = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export async function waitForRouter() {
	toasts.show('Restarting. This page comes back by itself.');
	await pause(20000);
	for (;;) {
		const alive = await fetch('/api/auth/session')
			.then((r) => r.ok)
			.catch(() => false);
		if (alive) return location.reload();
		await pause(3000);
	}
}

export async function send(kind: 'restore' | 'firmware', file: File) {
	const response = await fetch(`/api/system/${kind}`, { method: 'POST', body: file });
	if (!response.ok) {
		const payload = await response.json().catch(() => null);
		throw new ApiError(response.status, payload?.error ?? 'The router refused the file');
	}
	waitForRouter();
}
