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
 * Get Google OAuth credentials from calendar or google-tasks config.
 * Either component can supply credentials so users only need to configure once.
 */
export async function getGoogleOAuthConfig(): Promise<{
	clientId?: string;
	clientSecret?: string;
}> {
	const [calendarConfig, tasksConfig] = await Promise.all([
		getComponentConfig('calendar'),
		getComponentConfig('google-tasks')
	]);

	const config = calendarConfig.clientId
		? calendarConfig
		: (tasksConfig.clientId ? tasksConfig : calendarConfig);

	return {
		clientId: config.clientId as string | undefined,
		clientSecret: config.clientSecret as string | undefined
	};
}

/**
 * Get Google OAuth configuration (legacy compatibility)
 */
export async function getGoogleConfig() {
	const oauthConfig = await getGoogleOAuthConfig();
	const calendarConfig = await getComponentConfig('calendar');
	return {
		...oauthConfig,
		maxEvents: (calendarConfig.maxEvents as number | undefined) ?? 12,
		calendarColors: calendarConfig.calendarColors as
			| Array<{ calendarName: string; colorClass: string }>
			| Record<string, string>
			| undefined
	};
}

/**
 * Get Google Tasks configuration
 */
export async function getTasksConfig() {
	const oauthConfig = await getGoogleOAuthConfig();
	const tasksConfig = await getComponentConfig('google-tasks');
	return {
		clientId: oauthConfig.clientId,
		clientSecret: oauthConfig.clientSecret,
		maxTasks: (tasksConfig.maxTasks as number | undefined) ?? 20
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
