import type { PageServerLoad } from './$types';
import { loadConfig } from '$lib/core/config';
import { loadComponents } from '$lib/components/registry';

export const load: PageServerLoad = async () => {
	// Load components on server startup (for API handlers)
	await loadComponents();

	const config = await loadConfig();

	return {
		dashboardUserConfig: config.dashboard,
		componentConfigs: config.components
	};
};
