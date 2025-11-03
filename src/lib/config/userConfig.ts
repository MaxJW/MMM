import { getComponentConfig } from '$lib/core/config';

// Legacy compatibility layer - reads from new components structure
// These functions provide backward compatibility for endpoints that haven't been migrated yet

/**
 * Get weather configuration (legacy compatibility)
 */
export async function getWeatherConfig() {
	const config = await getComponentConfig('weather');
	return {
		apiKey: config.apiKey,
		latitude: config.latitude,
		longitude: config.longitude
	};
}

/**
 * Get Google OAuth configuration (legacy compatibility)
 */
export async function getGoogleConfig() {
	const config = await getComponentConfig('calendar'); // Google config is used by calendar component
	return {
		clientId: config.clientId as string | undefined,
		clientSecret: config.clientSecret as string | undefined,
		maxEvents: (config.maxEvents as number | undefined) ?? 12,
		calendarColors: config.calendarColors as
			| Array<{ calendarName: string; colorClass: string }>
			| Record<string, string>
			| undefined
	};
}

/**
 * Get Adguard configuration (legacy compatibility)
 */
export async function getAdguardConfig() {
	const config = await getComponentConfig('adguard');
	return {
		url: config.url,
		token: config.token
	};
}
