import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getConfig, saveConfig, clearConfigCache } from '$lib/config/userConfig';
import type { UserConfig } from '$lib/config/userConfig';
import { broadcastConfigChange } from '$lib/services/configStream';

export const GET: RequestHandler = async () => {
	try {
		const config = await getConfig();
		return json(config);
	} catch (error) {
		console.error('Error loading config:', error);
		return json({ error: 'Failed to load config' }, { status: 500 });
	}
};

export const PUT: RequestHandler = async ({ request }) => {
	try {
		const body = (await request.json()) as Partial<UserConfig>;
		const currentConfig = await getConfig();

		// Merge with existing config to preserve fields not in the update
		const updatedConfig: UserConfig = {
			...currentConfig,
			...body,
			dashboard: {
				components: body.dashboard?.components || currentConfig.dashboard.components
			},
			weather: {
				...currentConfig.weather,
				...body.weather
			},
			google: {
				...currentConfig.google,
				...body.google
			},
			spotify: {
				...currentConfig.spotify,
				...body.spotify
			},
			adguard: {
				...currentConfig.adguard,
				...body.adguard
			},
			energy: {
				...currentConfig.energy,
				...body.energy
			},
			rssFeeds: body.rssFeeds || currentConfig.rssFeeds,
			binCollections: {
				...currentConfig.binCollections,
				...body.binCollections
			},
			weatherAlerts: {
				...currentConfig.weatherAlerts,
				...body.weatherAlerts
			},
			wifi: {
				...currentConfig.wifi,
				...body.wifi
			}
		};

		await saveConfig(updatedConfig);
		// saveConfig already updates the cache, no need to clear it

		// Broadcast config change to all connected clients
		broadcastConfigChange();

		return json({ success: true, config: updatedConfig });
	} catch (error) {
		console.error('Error saving config:', error);
		return json({ error: 'Failed to save config' }, { status: 500 });
	}
};
