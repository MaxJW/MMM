import type { EnergyUsage } from '$lib/types/energy';

export class EnergyService {
	static async getUsage(): Promise<EnergyUsage | null> {
		try {
			const res = await fetch('/api/energy-usage');
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			return (await res.json()) as EnergyUsage;
		} catch (err) {
			console.error('Failed to fetch energy usage:', err);
			return null;
		}
	}
}
