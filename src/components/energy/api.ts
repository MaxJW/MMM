import type { EnergyUsage } from './types';
import { TIMING_STRATEGIES } from '$lib/core/timing';
import { getCached, setCache } from '$lib/core/utils';
import type { CacheEntry } from '$lib/core/utils';

let cache: CacheEntry<EnergyUsage> | null = null;

interface EnergyConfig {
	apiKey?: string;
}

async function fetchUsage(config: EnergyConfig): Promise<EnergyUsage> {
	if (!config.apiKey) {
		throw new Error('Missing Glowmarkt API key');
	}

	const headers = { Authorization: `Bearer ${config.apiKey}` };
	const todayRes = await fetch('https://api.glowmarkt.com/api/v0-1/resource/day', { headers });
	const monthRes = await fetch('https://api.glowmarkt.com/api/v0-1/resource/month', { headers });

	const today = await todayRes.json();
	const month = await monthRes.json();

	return {
		dayKWh: today.kwh,
		dayCost: today.cost,
		monthKWh: month.kwh,
		monthCost: month.cost
	};
}

export async function GET(config: EnergyConfig): Promise<EnergyUsage | { error: string }> {
	try {
		// Check cache
		const cached = getCached(cache);
		if (cached) {
			return cached;
		}

		const data = await fetchUsage(config);
		cache = setCache(cache, data, Date.now() + TIMING_STRATEGIES.STANDARD.interval);
		return data;
	} catch (error) {
		console.error('Energy API error:', error);
		return { error: error instanceof Error ? error.message : 'Failed to fetch energy usage' };
	}
}
