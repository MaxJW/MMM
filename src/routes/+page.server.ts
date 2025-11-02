import type { PageServerLoad } from './$types';
import { getDashboardConfig } from '$lib/core/config';
import { loadComponents } from '$lib/components/registry';

export const load: PageServerLoad = async () => {
	// Load components on server startup (for API handlers)
	await loadComponents();

	const dashboardUserConfig = await getDashboardConfig();

	return {
		dashboardUserConfig
	};
};