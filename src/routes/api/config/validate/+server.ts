import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getWeatherConfig, getAdguardConfig } from '$lib/config/userConfig';

export const GET: RequestHandler = async ({ url }) => {
	const service = url.searchParams.get('service');

	try {
		switch (service) {
			case 'weather': {
				const config = await getWeatherConfig();
				if (!config.apiKey || !config.latitude || !config.longitude) {
					return json({ valid: false, message: 'Missing required weather configuration' });
				}
				// Could test API connection here if needed
				return json({ valid: true });
			}
			case 'adguard': {
				const config = await getAdguardConfig();
				if (!config.url || !config.token) {
					return json({ valid: false, message: 'Missing Adguard URL or token' });
				}
				// Could test connection here if needed
				return json({ valid: true });
			}
			default:
				return json({ valid: false, message: 'Unknown service' }, { status: 400 });
		}
	} catch (error) {
		console.error('Validation error:', error);
		return json({ valid: false, message: 'Validation failed' }, { status: 500 });
	}
};
