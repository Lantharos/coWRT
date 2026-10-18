import { redirect } from '@sveltejs/kit';
import { api } from '#lib/api/client.js';
import type { SetupStatus } from '#lib/api/types.js';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async () => {
	const [session, setup] = await Promise.all([
		api.get<{ authenticated: boolean }>('/auth/session'),
		api.get<SetupStatus>('/setup')
	]);
	if (setup.needed && (session.authenticated || !setup.passwordSet)) redirect(307, '/setup');
	if (!session.authenticated) redirect(307, '/login');
};
