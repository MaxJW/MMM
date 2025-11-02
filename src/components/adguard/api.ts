import type { AdguardStats } from './types';
import { TIMING_STRATEGIES } from '$lib/core/timing';
import { getCached, setCache } from '$lib/core/utils';
import type { CacheEntry } from '$lib/core/utils';

let cache: CacheEntry<AdguardStats> | null = null;

interface AdguardConfig {
	url?: string;
	token?: string;
}

async function fetchStats(config: AdguardConfig): Promise<AdguardStats> {
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

export async function GET(config: AdguardConfig): Promise<AdguardStats | { error: string }> {
	try {
		// Check cache
		const cached = getCached(cache);
		if (cached) {
			return cached;
		}

		const data = await fetchStats(config);
		cache = setCache(cache, data, Date.now() + TIMING_STRATEGIES.FREQUENT.interval);
		return data;
	} catch (error) {
		console.error('Adguard API error:', error);
		return { error: error instanceof Error ? error.message : 'Failed to fetch Adguard stats' };
	}
}
