import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import {
	loadConfig,
	saveConfig,
	ensureAllComponentsInConfig,
	type UserConfig
} from '$lib/core/config';
import { broadcastConfigChange } from '$lib/services/configStream';

export const GET: RequestHandler = async () => {
	try {
		const config = await loadConfig();
		// Ensure all component manifests are included in the config
		const configWithAllComponents = await ensureAllComponentsInConfig(config);
		return json(configWithAllComponents);
	} catch (error) {
		console.error('Error loading config:', error);
		return json({ error: 'Failed to load config' }, { status: 500 });
	}
};

export const PUT: RequestHandler = async ({ request }) => {
	try {
		const body = (await request.json()) as Partial<UserConfig>;
		const currentConfig = await loadConfig();

		// Merge with existing config to preserve fields not in the update
		const updatedConfig: UserConfig = {
			...currentConfig,
			dashboard: {
				components: body.dashboard?.components || currentConfig.dashboard.components
			},
			components: {
				...currentConfig.components,
				...(body.components || {})
			}
		};

		// Ensure all component manifests are included before saving
		const configWithAllComponents = await ensureAllComponentsInConfig(updatedConfig);

		await saveConfig(configWithAllComponents);
		// saveConfig already updates the cache, no need to clear it

		// Broadcast config change to all connected clients
		broadcastConfigChange();

		return json({ success: true, config: configWithAllComponents });
	} catch (error) {
		console.error('Error saving config:', error);
		return json({ error: 'Failed to save config' }, { status: 500 });
	}
};
