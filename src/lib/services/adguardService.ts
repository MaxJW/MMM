import type { AdguardStats } from '$lib/types/adguard';

export class AdguardService {
	static async getStats(): Promise<AdguardStats | null> {
		try {
			const res = await fetch('/api/adguard');
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			return (await res.json()) as AdguardStats;
		} catch (err) {
			console.error('Failed to fetch Adguard stats:', err);
			return null;
		}
	}
}
