import dayjs from 'dayjs';

export class BinService {
	static async getNextBinCollection(): Promise<{ date: string; bins: string[] } | null> {
		try {
			const response = await fetch('/api/bin-collections');

			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			const data = await response.json();

			// If data is null, it means no upcoming collections
			if (data === null) {
				return null;
			}

			// Handle error response from our API (only if data is not null)
			if (data && data.error) {
				throw new Error(data.error);
			}

			// Validate the data structure
			if (!data || typeof data !== 'object' || !data.date || !Array.isArray(data.bins)) {
				console.warn('Invalid bin collection data received:', data);
				return null;
			}

			return data;
		} catch (error) {
			console.error('Failed to fetch bin collections:', error);
			return null;
		}
	}

	static formatDate(dateString: string): string {
		const date = dayjs(dateString);
		const today = dayjs();
		const tomorrow = today.add(1, 'day');

		if (date.isSame(today, 'day')) {
			return 'Today';
		} else if (date.isSame(tomorrow, 'day')) {
			return 'Tomorrow';
		} else {
			return date.format('dddd, D MMMM');
		}
	}
}
