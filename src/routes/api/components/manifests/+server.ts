import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { loadComponents, getComponents } from '$lib/components/registry';

export const GET: RequestHandler = async () => {
	try {
		await loadComponents();
		const components = getComponents();

		// Return just the manifests (without the component/API handler references)
		const manifests = components.map((comp) => ({
			id: comp.id,
			manifest: comp.manifest
		}));

		return json({ components: manifests });
	} catch (error) {
		console.error('Error loading component manifests:', error);
		return json({ error: 'Failed to load component manifests', components: [] }, { status: 500 });
	}
};
