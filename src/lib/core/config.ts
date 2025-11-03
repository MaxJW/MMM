import { readFile, writeFile, mkdir } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import type { DashboardArea } from './types';
import { loadComponents, getComponentIds } from '$lib/components/registry';

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
	components: Record<string, Record<string, unknown>>;
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
 * Ensure all component manifests are represented in the dashboard config
 * - Removes component IDs that don't match any existing component
 * - Adds missing components with enabled: false and area: 'top-left'
 * - Automatically saves the cleaned config to file if invalid components were removed
 */
export async function ensureAllComponentsInConfig(config: UserConfig): Promise<UserConfig> {
	try {
		// Only load components if not already loaded (optimization)
		await loadComponents();
		// Use cached component IDs set for better performance
		const validComponentIds = getComponentIds();

		// Filter out invalid component IDs (components that no longer exist)
		const validComponents = config.dashboard.components.filter((comp) =>
			validComponentIds.has(comp.id)
		);

		// Check if any invalid components were removed
		const removedComponents = config.dashboard.components.filter(
			(comp) => !validComponentIds.has(comp.id)
		);

		// Create a set of existing valid component IDs
		const existingIds = new Set(validComponents.map((comp) => comp.id));

		// Find missing component IDs
		const missingIds = Array.from(validComponentIds).filter((id) => !existingIds.has(id));

		// Add missing components with default values
		const missingComponents: DashboardComponentConfig[] = missingIds.map((id) => ({
			id,
			enabled: false,
			area: 'top-left' as DashboardArea
		}));

		// Build the cleaned config
		const cleanedConfig: UserConfig = {
			...config,
			dashboard: {
				components: [...validComponents, ...missingComponents]
			}
		};

		// If invalid components were removed, save the cleaned config to file
		if (removedComponents.length > 0) {
			console.log(
				`Removed ${removedComponents.length} invalid component(s) from config:`,
				removedComponents.map((c) => c.id).join(', ')
			);
			try {
				await saveConfig(cleanedConfig);
				console.log('Config file updated with cleaned configuration');
			} catch (saveError) {
				console.error('Error saving cleaned config:', saveError);
				// Continue even if save fails
			}
		}

		if (missingIds.length > 0) {
			console.log(
				`Added ${missingIds.length} missing component(s) to config:`,
				missingIds.join(', ')
			);
			// Save when missing components are added too, but only if we haven't already saved
			if (removedComponents.length === 0) {
				try {
					await saveConfig(cleanedConfig);
					console.log('Config file updated with missing components');
				} catch (saveError) {
					console.error('Error saving config with missing components:', saveError);
					// Continue even if save fails
				}
			}
		}

		return cleanedConfig;
	} catch (error) {
		console.error('Error ensuring all components in config:', error);
		// Return original config if there's an error
		return config;
	}
}

/**
 * Get dashboard configuration
 */
export async function getDashboardConfig() {
	const config = await loadConfig();
	return config.dashboard;
}

/**
 * Get component configuration
 */
export async function getComponentConfig(componentId: string) {
	const config = await loadConfig();
	return config.components[componentId] ?? {};
}

/**
 * Clear config cache (useful after saving)
 */
export function clearConfigCache() {
	cachedConfig = null;
}

export { CONFIG_DIR, CONFIG_FILE };
