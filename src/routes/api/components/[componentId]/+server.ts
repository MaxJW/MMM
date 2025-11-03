import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { callComponentApi } from '$lib/components/api-proxy';
import { loadComponents } from '$lib/components/registry';

export const GET: RequestHandler = async ({ params, request }) => {
	const { componentId } = params;

	if (!componentId) {
		return json({ error: 'Component ID required' }, { status: 400 });
	}

	// Ensure components are loaded before calling the API
	await loadComponents();

	const result = await callComponentApi(componentId, 'GET', request);

	if (result.error) {
		return json({ error: result.error }, { status: result.status || 500 });
	}

	return json(result.data);
};

export const POST: RequestHandler = async ({ params, request }) => {
	const { componentId } = params;

	if (!componentId) {
		return json({ error: 'Component ID required' }, { status: 400 });
	}

	const result = await callComponentApi(componentId, 'POST', request);

	if (result.error) {
		return json({ error: result.error }, { status: result.status || 500 });
	}

	return json(result.data);
};

export const PUT: RequestHandler = async ({ params, request }) => {
	const { componentId } = params;

	if (!componentId) {
		return json({ error: 'Component ID required' }, { status: 400 });
	}

	const result = await callComponentApi(componentId, 'PUT', request);

	if (result.error) {
		return json({ error: result.error }, { status: result.status || 500 });
	}

	return json(result.data);
};

export const DELETE: RequestHandler = async ({ params, request }) => {
	const { componentId } = params;

	if (!componentId) {
		return json({ error: 'Component ID required' }, { status: 400 });
	}

	const result = await callComponentApi(componentId, 'DELETE', request);

	if (result.error) {
		return json({ error: result.error }, { status: result.status || 500 });
	}

	return json(result.data);
};
