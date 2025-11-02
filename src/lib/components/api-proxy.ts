import type { Component } from './types';
import { getComponent } from './registry';
import { getComponentConfig } from '$lib/core/config';

/**
 * Call a component's API handler
 * Returns the result directly (not a Response object)
 */
export async function callComponentApi(
	componentId: string,
	method: string = 'GET',
	request?: Request
): Promise<{ data?: any; error?: string; status?: number }> {
	const component = getComponent(componentId);

	if (!component) {
		return { error: `Component ${componentId} not found`, status: 404 };
	}

	if (!component.apiHandler) {
		return { error: `Component ${componentId} has no API handler`, status: 404 };
	}

	try {
		const config = await getComponentConfig(componentId);
		const result = await component.apiHandler(config, request);

		// If result has an error, return it with 500 status
		if (result && typeof result === 'object' && 'error' in result) {
			return { error: result.error, status: 500 };
		}

		return { data: result, status: 200 };
	} catch (error) {
		console.error(`Error calling API for component ${componentId}:`, error);
		return {
			error: error instanceof Error ? error.message : 'Unknown error',
			status: 500
		};
	}
}
