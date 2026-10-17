import { toasts } from '#lib/toast.svelte.js';
import type { Applied } from './types';

const RETRY_MS = 1500;

async function confirm({ token, seconds }: NonNullable<Applied['confirm']>) {
	const deadline = Date.now() + (seconds - 5) * 1000;
	while (Date.now() < deadline) {
		await new Promise((resolve) => setTimeout(resolve, RETRY_MS));
		const response = await fetch(`/api/changes/${token}/confirm`, { method: 'POST' }).catch(
			() => null
		);
		if (response?.ok) return true;
		if (response?.status === 409) return false;
	}
	return false;
}

export async function applyChange(request: () => Promise<Applied>, done: string) {
	const applied = await request();
	if (!applied.confirm) {
		toasts.show(done);
		return true;
	}
	toasts.show('Applying, making sure you can still reach the router');
	const confirmed = await confirm(applied.confirm);
	if (confirmed) toasts.show(done);
	else toasts.show("That change cut the router off, so it's being undone", 'error');
	return confirmed;
}
