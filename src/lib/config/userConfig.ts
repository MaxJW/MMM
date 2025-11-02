import { readFile, writeFile, mkdir } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import type { DashboardArea } from '$lib/types/dashboard';

const CONFIG_DIR = join(process.cwd(), 'data');
const CONFIG_FILE = join(CONFIG_DIR, 'config.json');

export interface DashboardComponentConfig {
	id: string;
	enabled: boolean;
	area: DashboardArea;
}

export interface UserConfig {
	dashboard: {
		components: DashboardComponentConfig[];
	};
	weather: {
		apiKey?: string;
		latitude?: string;
		longitude?: string;
	};
	google: {
		clientId?: string;
		clientSecret?: string;
	};
	spotify: {
		clientId?: string;
		clientSecret?: string;
		refreshToken?: string;
	};
	adguard: {
		url?: string;
		token?: string;
	};
	energy: {
		apiKey?: string;
	};
	rssFeeds: Array<{ url: string; sourceName: string }>;
	binCollections: {
		uprn?: string;
		apiEndpoint?: string;
		council?: string;
	};
	weatherAlerts: {
		country?: string;
		province?: string;
	};
	wifi: {
		networkName?: string;
		password?: string;
		securityType?: 'WPA' | 'WEP' | 'nopass';
	};
}

let cachedConfig: UserConfig | null = null;

/**
 * Get default configuration matching current hardcoded values
 */
function getDefaultConfig(): UserConfig {
	return {
		dashboard: {
			components: [
				{ id: 'clock', enabled: true, area: 'top-left' },
				{ id: 'weather', enabled: true, area: 'top-right' },
				{ id: 'events', enabled: true, area: 'top-left' },
				{ id: 'system-stats', enabled: true, area: 'bottom-right' },
				{ id: 'wifi-qr-code', enabled: true, area: 'bottom-left' },
				{ id: 'event-image', enabled: true, area: 'center' },
				{ id: 'greetings', enabled: true, area: 'center' },
				{ id: 'reminders', enabled: true, area: 'notifications' },
				{ id: 'rss-feed', enabled: true, area: 'notifications' },
				{ id: 'adguard', enabled: true, area: 'middle-right' },
				{ id: 'energy', enabled: false, area: 'middle-right' },
				{ id: 'spotify', enabled: false, area: 'middle-right' }
			]
		},
		weather: {},
		google: {},
		spotify: {},
		adguard: {},
		energy: {},
		rssFeeds: [
			{ url: 'https://feeds.skynews.com/feeds/rss/home.xml', sourceName: 'Sky News' },
			{ url: 'https://www.thecourier.co.uk/feed/', sourceName: 'The Courier' },
			{ url: 'https://www.fife.gov.uk/news/rss/latest', sourceName: 'Fife Council' },
			{ url: 'https://feeds.arstechnica.com/arstechnica/index/', sourceName: 'Ars Technica' },
			{ url: 'https://www.theguardian.com/world/rss', sourceName: 'The Guardian | World' },
			{ url: 'https://www.theguardian.com/uk-news/rss', sourceName: 'The Guardian | UK' }
		],
		binCollections: {},
		weatherAlerts: {
			country: 'united-kingdom',
			province: 'Central, Tayside & Fife'
		},
		wifi: {}
	};
}

/**
 * Merge user config with defaults
 */
function mergeWithDefaults(config: Partial<UserConfig>): UserConfig {
	const defaults = getDefaultConfig();

	const merged: UserConfig = {
		dashboard: {
			components: config.dashboard?.components || defaults.dashboard.components
		},
		weather: {
			apiKey: config.weather?.apiKey || defaults.weather.apiKey,
			latitude: config.weather?.latitude || defaults.weather.latitude,
			longitude: config.weather?.longitude || defaults.weather.longitude
		},
		google: {
			clientId: config.google?.clientId || defaults.google.clientId,
			clientSecret: config.google?.clientSecret || defaults.google.clientSecret
		},
		spotify: {
			clientId: config.spotify?.clientId || defaults.spotify.clientId,
			clientSecret: config.spotify?.clientSecret || defaults.spotify.clientSecret,
			refreshToken: config.spotify?.refreshToken || defaults.spotify.refreshToken
		},
		adguard: {
			url: config.adguard?.url || defaults.adguard.url,
			token: config.adguard?.token || defaults.adguard.token
		},
		energy: {
			apiKey: config.energy?.apiKey || defaults.energy.apiKey
		},
		rssFeeds: config.rssFeeds || defaults.rssFeeds,
		binCollections: {
			uprn: config.binCollections?.uprn || defaults.binCollections.uprn,
			apiEndpoint: config.binCollections?.apiEndpoint || defaults.binCollections.apiEndpoint,
			council: config.binCollections?.council || defaults.binCollections.council
		},
		weatherAlerts: {
			country: config.weatherAlerts?.country || defaults.weatherAlerts.country,
			province: config.weatherAlerts?.province || defaults.weatherAlerts.province
		},
		wifi: {
			networkName: config.wifi?.networkName || defaults.wifi.networkName,
			password: config.wifi?.password || defaults.wifi.password,
			securityType: config.wifi?.securityType || defaults.wifi.securityType || 'WPA'
		}
	};

	return merged;
}

/**
 * Initialize config directory if it doesn't exist
 */
async function ensureConfigDir() {
	if (!existsSync(CONFIG_DIR)) {
		await mkdir(CONFIG_DIR, { recursive: true });
	}
}

/**
 * Load configuration from file
 */
export async function loadConfig(): Promise<UserConfig> {
	if (cachedConfig) {
		return cachedConfig;
	}

	try {
		await ensureConfigDir();

		if (!existsSync(CONFIG_FILE)) {
			// Return defaults if config file doesn't exist
			cachedConfig = mergeWithDefaults({});
			return cachedConfig;
		}

		const data = await readFile(CONFIG_FILE, 'utf8');
		const parsed = JSON.parse(data) as Partial<UserConfig>;
		cachedConfig = mergeWithDefaults(parsed);
		return cachedConfig;
	} catch (error) {
		console.error('Error loading config:', error);
		// Return defaults on error
		cachedConfig = mergeWithDefaults({});
		return cachedConfig;
	}
}

/**
 * Save configuration to file
 */
export async function saveConfig(config: UserConfig): Promise<void> {
	try {
		await ensureConfigDir();
		await writeFile(CONFIG_FILE, JSON.stringify(config, null, 2));
		cachedConfig = config; // Update cache
	} catch (error) {
		console.error('Error saving config:', error);
		throw error;
	}
}

/**
 * Get current configuration (loads if not cached)
 */
export async function getConfig(): Promise<UserConfig> {
	return await loadConfig();
}

/**
 * Get weather configuration
 */
export async function getWeatherConfig() {
	const config = await getConfig();
	return config.weather;
}

/**
 * Get Google OAuth configuration
 */
export async function getGoogleConfig() {
	const config = await getConfig();
	return config.google;
}

/**
 * Get Spotify configuration
 */
export async function getSpotifyConfig() {
	const config = await getConfig();
	return config.spotify;
}

/**
 * Get Adguard configuration
 */
export async function getAdguardConfig() {
	const config = await getConfig();
	return config.adguard;
}

/**
 * Get Energy API configuration
 */
export async function getEnergyConfig() {
	const config = await getConfig();
	return config.energy;
}

/**
 * Get RSS feeds configuration
 */
export async function getRSSFeedsConfig() {
	const config = await getConfig();
	return config.rssFeeds;
}

/**
 * Get bin collections configuration
 */
export async function getBinCollectionsConfig() {
	const config = await getConfig();
	return config.binCollections;
}

/**
 * Get weather alerts configuration
 */
export async function getWeatherAlertsConfig() {
	const config = await getConfig();
	return config.weatherAlerts;
}

/**
 * Get WiFi configuration
 */
export async function getWiFiConfig() {
	const config = await getConfig();
	return config.wifi;
}

/**
 * Get dashboard configuration
 */
export async function getDashboardConfig() {
	const config = await getConfig();
	return config.dashboard;
}

/**
 * Clear config cache (useful after saving)
 */
export function clearConfigCache() {
	cachedConfig = null;
}
