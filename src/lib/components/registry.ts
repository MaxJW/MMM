import { readdir, readFile } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import type { Component, ComponentManifest } from './types';

const COMPONENTS_DIR = join(process.cwd(), 'src', 'components');
const PLUGINS_DIR = join(process.cwd(), 'plugins');

interface ComponentRegistry {
	components: Map<string, Component>;
	componentIds: Set<string>; // Cached set of component IDs for fast lookups
	loaded: boolean;
	loadingPromise: Promise<void> | null; // Promise for concurrent calls to await
}

const registry: ComponentRegistry = {
	components: new Map(),
	componentIds: new Set(),
	loaded: false,
	loadingPromise: null
};

/**
 * Load a component's manifest.json file
 */
async function loadManifest(componentPath: string): Promise<ComponentManifest | null> {
	const manifestPath = join(componentPath, 'manifest.json');

	if (!existsSync(manifestPath)) {
		console.warn(`No manifest.json found for component at ${componentPath}`);
		return null;
	}

	try {
		const manifestContent = await readFile(manifestPath, 'utf8');
		const manifest = JSON.parse(manifestContent) as ComponentManifest;

		// Validate manifest structure
		if (!manifest.id || !manifest.name || !manifest.config) {
			console.error(`Invalid manifest structure for component at ${componentPath}`);
			return null;
		}

		return manifest;
	} catch (error) {
		console.error(`Error loading manifest from ${manifestPath}:`, error);
		return null;
	}
}

/**
 * Load a component's API handler
 * For built-in components: uses static imports
 * For external plugins: uses dynamic imports
 */
async function loadApiHandler(
	componentPath: string,
	componentId: string,
	isExternal: boolean = false
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
): Promise<((config: any, request?: Request) => Promise<any>) | undefined> {
	const apiFile = join(componentPath, 'api.ts');
	const apiJsFile = join(componentPath, 'api.js');

	// Check for TypeScript or JavaScript API file
	const apiFilePath = existsSync(apiFile) ? apiFile : existsSync(apiJsFile) ? apiJsFile : null;

	if (!apiFilePath) {
		return undefined;
	}

	try {
		if (isExternal) {
			// For external plugins, use dynamic import
			// Bun supports direct file path imports
			const module = await import(apiFilePath);

			// Support both default export and named GET export
			const handler = module.default || module.GET;

			if (handler && typeof handler === 'function') {
				return handler;
			}

			return undefined;
		} else {
			// For built-in components, use static imports from the api-handlers module
			const { apiHandlerMap } = await import('../../components/api-handlers');

			// Use the manifest ID to look up the handler (not the directory name)
			const handler = apiHandlerMap[componentId];

			if (handler && typeof handler === 'function') {
				return handler;
			}

			return undefined;
		}
	} catch (error) {
		console.error(`Error loading API handler from ${apiFilePath}:`, error);
		// Return undefined to indicate no API handler, component can still work without it
		return undefined;
	}
}

/**
 * Load a single component
 */
async function loadComponent(
	componentDir: string,
	baseDir: string,
	isExternal: boolean = false
): Promise<Component | null> {
	const componentPath = join(baseDir, componentDir);

	if (!existsSync(componentPath)) {
		return null;
	}

	const manifest = await loadManifest(componentPath);
	if (!manifest) {
		return null;
	}

	const component: Component = {
		id: manifest.id,
		manifest,
		apiHandler: await loadApiHandler(componentPath, manifest.id, isExternal)
	};

	return component;
}

/**
 * Discover and load all components
 * This function is idempotent and safe to call concurrently - multiple calls will await the same loading promise
 */
export async function loadComponents(): Promise<void> {
	// If already loaded, return immediately
	if (registry.loaded) {
		return;
	}

	// If loading is in progress, await the existing promise
	if (registry.loadingPromise) {
		return registry.loadingPromise;
	}

	// Start loading and store the promise for concurrent calls
	registry.loadingPromise = (async () => {
		try {
			// Load built-in components first
			if (existsSync(COMPONENTS_DIR)) {
				const entries = await readdir(COMPONENTS_DIR, { withFileTypes: true });

				for (const entry of entries) {
					if (entry.isDirectory()) {
						const component = await loadComponent(entry.name, COMPONENTS_DIR, false);
						if (component) {
							// Built-in components take precedence
							if (registry.components.has(component.id)) {
								console.warn(
									`Component ID "${component.id}" already exists in registry. Overwriting with built-in component from ${entry.name}`
								);
							}
							registry.components.set(component.id, component);
							registry.componentIds.add(component.id); // Cache ID for fast lookups
						}
					}
				}
			} else {
				console.warn(`Components directory not found: ${COMPONENTS_DIR}`);
			}

			// Load external plugins (these won't overwrite built-in components)
			if (existsSync(PLUGINS_DIR)) {
				const pluginEntries = await readdir(PLUGINS_DIR, { withFileTypes: true });

				for (const entry of pluginEntries) {
					if (entry.isDirectory()) {
						const component = await loadComponent(entry.name, PLUGINS_DIR, true);
						if (component) {
							// External plugins won't overwrite built-in components
							if (registry.components.has(component.id)) {
								console.warn(
									`Plugin component ID "${component.id}" conflicts with built-in component. Skipping plugin from ${entry.name}`
								);
								continue;
							}
							registry.components.set(component.id, component);
							registry.componentIds.add(component.id);
							console.log(`Loaded external plugin: ${component.id}`);
						}
					}
				}
			} else {
				// Plugins directory doesn't exist yet, that's okay
				console.log(
					`Plugins directory not found: ${PLUGINS_DIR} (external plugins will be loaded from here if added)`
				);
			}

			registry.loaded = true;
			console.log(`Loaded ${registry.components.size} components total`);
		} catch (error) {
			console.error('Error loading components:', error);
			registry.loaded = true;
		} finally {
			// Clear the loading promise so future calls can start fresh if needed
			registry.loadingPromise = null;
		}
	})();

	return registry.loadingPromise;
}

/**
 * Get all registered components
 */
export function getComponents(): Component[] {
	return Array.from(registry.components.values());
}

/**
 * Get all registered component IDs (cached for performance)
 */
export function getComponentIds(): Set<string> {
	return registry.componentIds;
}

/**
 * Get a component by ID
 */
export function getComponent(id: string): Component | undefined {
	return registry.components.get(id);
}

/**
 * Clear the component registry (useful for testing or hot reloading)
 */
export function clearRegistry() {
	registry.components.clear();
	registry.componentIds.clear();
	registry.loaded = false;
	registry.loadingPromise = null;
}
