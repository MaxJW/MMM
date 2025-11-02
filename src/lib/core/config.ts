import { readFile, writeFile, mkdir } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import type { DashboardArea } from './types';

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
	components: Record<string, any>;
}

let cachedConfig: UserConfig | null = null;

/**
 * Initialize config directory if it doesn't exist
 */
async function ensureConfigDir() {
	if (!existsSync(CONFIG_DIR)) {
		await mkdir(CONFIG_DIR, { recursive: true });
	}
}

/**
 * Get default configuration
 */
function getDefaultConfig(): UserConfig {
	return {
		dashboard: {
			components: [
				{ id: 'clock', enabled: true, area: 'top-left' },
				{ id: 'weather', enabled: true, area: 'top-right' },
			{ id: 'calendar', enabled: true, area: 'top-left' },
			{ id: 'system-stats', enabled: true, area: 'bottom-right' },
			{ id: 'wifi-qr-code', enabled: true, area: 'bottom-left' },
			{ id: 'events', enabled: true, area: 'center' },
				{ id: 'greetings', enabled: true, area: 'center' },
				{ id: 'reminders', enabled: true, area: 'notifications' },
				{ id: 'rss-feed', enabled: true, area: 'notifications' },
				{ id: 'adguard', enabled: true, area: 'middle-right' },
				{ id: 'energy', enabled: false, area: 'middle-right' },
				{ id: 'spotify', enabled: false, area: 'middle-right' }
			]
		},
		components: {}
	};
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
			cachedConfig = getDefaultConfig();
			return cachedConfig;
		}

		const data = await readFile(CONFIG_FILE, 'utf8');
		const parsed = JSON.parse(data) as Partial<UserConfig>;

		// Merge with defaults
		const defaults = getDefaultConfig();
		cachedConfig = {
			dashboard: parsed.dashboard ?? defaults.dashboard,
			components: parsed.components ?? defaults.components
		};

		return cachedConfig;
	} catch (error) {
		console.error('Error loading config:', error);
		cachedConfig = getDefaultConfig();
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
		cachedConfig = config;
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
 * Get dashboard configuration
 */
export async function getDashboardConfig() {
	const config = await getConfig();
	return config.dashboard;
}

/**
 * Get component configuration
 */
export async function getComponentConfig(componentId: string) {
	const config = await getConfig();
	return config.components[componentId] ?? {};
}

/**
 * Clear config cache (useful after saving)
 */
export function clearConfigCache() {
	cachedConfig = null;
}

export { CONFIG_DIR, CONFIG_FILE };
