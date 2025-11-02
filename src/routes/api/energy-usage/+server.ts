import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getEnergyConfig } from '$lib/config/userConfig';
import type { EnergyUsage } from '$lib/types/energy';
import { TIMING_STRATEGIES } from '$lib/types/util';

class EnergyApi {
	private static cache: { data: EnergyUsage; expiry: number } | null = null;

	private static async fetchUsage(): Promise<EnergyUsage> {
		const config = await getEnergyConfig();
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

	static async getUsage(): Promise<EnergyUsage> {
		if (this.cache && Date.now() < this.cache.expiry) return this.cache.data;
		const data = await this.fetchUsage();
		this.cache = { data, expiry: Date.now() + TIMING_STRATEGIES.STANDARD.interval };
		return data;
	}
}

export const GET: RequestHandler = async () => {
	try {
		return json(await EnergyApi.getUsage());
	} catch (err) {
		console.error('Energy API error:', err);
		return json({ error: 'Failed to fetch energy usage' }, { status: 500 });
	}
};
