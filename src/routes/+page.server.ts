import type { PageServerLoad } from './$types';
import { getDashboardConfig } from '$lib/config/userConfig';

export const load: PageServerLoad = async () => {
	const dashboardUserConfig = await getDashboardConfig();

	return {
		dashboardUserConfig
	};
};

