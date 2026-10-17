import { redirect } from '@sveltejs/kit';
import { api } from '#lib/api/client.js';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async () => {
	const session = await api.get<{ authenticated: boolean }>('/auth/session');
	if (!session.authenticated) redirect(307, '/login');
};
