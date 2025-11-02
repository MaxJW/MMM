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
 * Filters enabled components and maps IDs to components
 * Preserves the order from the JSON array (components appear in the order they're listed)
 *
 * This function is safe to call client-side (doesn't use Node.js modules)
 */
export async function buildDashboardConfig(userConfig: {
	components: DashboardComponentConfig[];
}): Promise<DashboardConfig> {
	// Get component map from explicit imports (more reliable than glob)
	const componentMap = getComponentMap();

	// Filter enabled components and preserve array order
	const enabledComponents = userConfig.components.filter((comp) => comp.enabled);

	const components = enabledComponents
		.map((comp) => {
			const component = componentMap[comp.id];
			if (!component) {
				console.warn(`Component ${comp.id} not found in componentMap`);
				return null;
			}
			return {
				id: comp.id,
				component,
				area: comp.area as DashboardArea
			};
		})
		.filter((comp): comp is NonNullable<typeof comp> => comp !== null);

	return { components };
}
