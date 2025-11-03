import type { DashboardConfig, DashboardArea } from '$lib/core/types';
import type { DashboardComponentConfig } from '$lib/core/config';
import { componentMap } from '../../components/index';

/**
 * Get the component map - uses explicit imports from components/index.ts
 * This is more reliable than glob patterns and better for build tools
 */
function getComponentMap(): Record<string, unknown> {
	return componentMap;
}

/**
 * Build dashboard config from user config
 * Includes all components from componentMap:
 * - Components in config with enabled: true appear in their configured area
 * - Components in config with enabled: false appear in top-left (turned off)
 * - Components not in config appear in top-left (turned off)
 * Preserves the order from the JSON array for enabled components
 *
 * This function is safe to call client-side (doesn't use Node.js modules)
 */
export async function buildDashboardConfig(userConfig: {
	components: DashboardComponentConfig[];
}): Promise<DashboardConfig> {
	// Get component map from explicit imports (more reliable than glob)
	const componentMap = getComponentMap();

	// Create a map of user config components by ID for quick lookup
	const configMap = new Map<string, DashboardComponentConfig>();
	for (const comp of userConfig.components) {
		configMap.set(comp.id, comp);
	}

	const components: Array<{
		id: string;
		component: unknown;
		area: DashboardArea;
		enabled: boolean;
	}> = [];

	// First, add all enabled components from config in their configured areas
	for (const compConfig of userConfig.components) {
		if (compConfig.enabled) {
			const component = componentMap[compConfig.id];
			if (!component) {
				console.warn(`Component ${compConfig.id} not found in componentMap`);
				continue;
			}
			components.push({
				id: compConfig.id,
				component,
				area: compConfig.area as DashboardArea,
				enabled: true
			});
		}
	}

	// Then, add all components from componentMap that aren't in config or are disabled
	for (const [componentId, component] of Object.entries(componentMap)) {
		const config = configMap.get(componentId);

		// If component is not in config, add it to top-left with enabled: false
		if (!config) {
			components.push({
				id: componentId,
				component,
				area: 'top-left',
				enabled: false
			});
		}
		// If component is in config but disabled, add it to top-left with enabled: false
		else if (!config.enabled) {
			components.push({
				id: componentId,
				component,
				area: 'top-left',
				enabled: false
			});
		}
	}

	return { components };
}
