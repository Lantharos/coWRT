import { goto } from '$app/navigation';

export class ApiError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
	const response = await fetch(`/api${path}`, {
		method,
		headers: body === undefined ? undefined : { 'content-type': 'application/json' },
		body: body === undefined ? undefined : JSON.stringify(body)
	});
	if (response.status === 401 && !path.startsWith('/auth/')) {
		await goto('/login');
	}
	if (!response.ok) {
		const payload = await response.json().catch(() => null);
		throw new ApiError(response.status, payload?.error ?? `Request failed (${response.status})`);
	}
	const text = await response.text();
	return (text ? JSON.parse(text) : undefined) as T;
}

export const api = {
	get: <T>(path: string) => request<T>('GET', path),
	post: <T>(path: string, body: unknown = {}) => request<T>('POST', path, body),
	put: <T>(path: string, body: unknown) => request<T>('PUT', path, body),
	patch: <T>(path: string, body: unknown) => request<T>('PATCH', path, body),
	delete: <T>(path: string) => request<T>('DELETE', path)
};
