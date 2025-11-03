import { readdir, readFile, stat } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import type { Component, ComponentManifest } from './types';

const COMPONENTS_DIR = join(process.cwd(), 'src', 'components');

interface ComponentRegistry {
	components: Map<string, Component>;
	loaded: boolean;
}

const registry: ComponentRegistry = {
	components: new Map(),
	loaded: false
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
 * Load component module - components will be loaded dynamically via import.meta.glob
 * This is handled in dashboard.ts when building the dashboard config
 */
async function loadComponentModule(componentPath: string): Promise<any | null> {
	// Component loading is handled dynamically via import.meta.glob in buildDashboardConfig
	// We'll return null here and handle it during dashboard config building
	return null;
}

/**
 * Load a component's API handler using static imports
 * This is more reliable than dynamic file:// imports and works in both dev and production
 */
async function loadApiHandler(
	componentPath: string,
	componentDir: string
): Promise<((config: any, request?: Request) => Promise<any>) | null> {
	const apiFile = join(componentPath, 'api.ts');

	if (!existsSync(apiFile)) {
		return null;
	}

	try {
		// Use static imports from the api-handlers module
		// This ensures handlers are available at runtime in both dev and production
		const { apiHandlerMap } = await import('../../components/api-handlers');

		// Get the component ID from the directory name
		const componentId = componentDir;
		const handler = apiHandlerMap[componentId];

		if (handler && typeof handler === 'function') {
			return handler;
		}

		return null;
	} catch (error) {
		console.error(`Error loading API handler from ${apiFile}:`, error);
		// Return null to indicate no API handler, component can still work without it
		return null;
	}
}

/**
 * Load a single component
 */
async function loadComponent(componentDir: string): Promise<Component | null> {
	const componentPath = join(COMPONENTS_DIR, componentDir);

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
		apiHandler: await loadApiHandler(componentPath, componentDir)
	};

	return component;
}

/**
 * Discover and load all components
 */
export async function loadComponents(): Promise<void> {
	if (registry.loaded) {
		return;
	}

	if (!existsSync(COMPONENTS_DIR)) {
		console.warn(`Components directory not found: ${COMPONENTS_DIR}`);
		registry.loaded = true;
		return;
	}

	try {
		const entries = await readdir(COMPONENTS_DIR, { withFileTypes: true });

		for (const entry of entries) {
			if (entry.isDirectory()) {
				const component = await loadComponent(entry.name);
				if (component) {
					registry.components.set(component.id, component);
				}
			}
		}

		registry.loaded = true;
		console.log(`Loaded ${registry.components.size} components`);
	} catch (error) {
		console.error('Error loading components:', error);
		registry.loaded = true;
	}
}

/**
 * Get all registered components
 */
export function getComponents(): Component[] {
	return Array.from(registry.components.values());
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
	registry.loaded = false;
}
