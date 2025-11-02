import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getAdguardConfig } from '$lib/config/userConfig';
import type { AdguardStats } from '$lib/types/adguard';
import { TIMING_STRATEGIES } from '$lib/types/util';

class AdguardApi {
	private static cache: { data: AdguardStats; expiry: number } | null = null;

	private static async fetchStats(): Promise<AdguardStats> {
		const config = await getAdguardConfig();
		if (!config.url || !config.token) {
			throw new Error('Missing Adguard URL or token');
		}

		const res = await fetch(`${config.url}/control/stats`, {
			headers: { Authorization: `Basic ${config.token}` }
		});
		if (!res.ok) throw new Error(`HTTP ${res.status}`);
		const data = await res.json();
		return { queries: data.num_dns_queries, blocked: data.num_blocked_filtering };
	}

	static async getStats(): Promise<AdguardStats> {
		if (this.cache && Date.now() < this.cache.expiry) return this.cache.data;
		const data = await this.fetchStats();
		this.cache = { data, expiry: Date.now() + TIMING_STRATEGIES.FREQUENT.interval };
		return data;
	}
}

export const GET: RequestHandler = async () => {
	try {
		return json(await AdguardApi.getStats());
	} catch (err) {
		console.error('Adguard API error:', err);
		return json({ error: 'Failed to fetch Adguard stats' }, { status: 500 });
	}
};
