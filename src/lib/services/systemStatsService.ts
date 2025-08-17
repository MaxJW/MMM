import type { SystemStats } from '$lib/types/system-stats';

export class SystemStatsService {
	static async getSystemStats(): Promise<SystemStats | null> {
		try {
			const response = await fetch('/api/system-stats');

			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			const data = await response.json();

			// Handle error response from our API
			if (data && data.error) {
				throw new Error(data.error);
			}

			// Validate the data structure
			if (!data || typeof data !== 'object' || typeof data.cpu !== 'number') {
				console.warn('Invalid system stats data received:', data);
				return null;
			}

			return data as SystemStats;
		} catch (error) {
			console.error('Failed to fetch system stats:', error);
			return null;
		}
	}
}
