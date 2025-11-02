import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import dayjs from 'dayjs';
import { TIMING_STRATEGIES } from '$lib/types/util';
import type { BinApiResponse } from '$lib/types/bin';
import { getBinCollectionsConfig } from '$lib/config/userConfig';

interface BinCollectionRaw {
	date: string;
	colour: string;
}

class BinService {
	private static authToken: string | null = null;
	private static tokenExpiry: number = 0;
	private static cache: { collections: BinCollectionRaw[]; expiry: number } | null = null;

	private static async getAuthToken(apiEndpoint?: string): Promise<string | null> {
		if (this.authToken && Date.now() < this.tokenExpiry) {
			return this.authToken;
		}

		// Default to Fife Council endpoint if not configured
		const authUrl = apiEndpoint
			? `${apiEndpoint}/api/citizen?preview=false&locale=en`
			: 'https://www.fife.gov.uk/api/citizen?preview=false&locale=en';

		try {
			const response = await fetch(authUrl);
			const token = response.headers.get('Authorization');

			if (token) {
				this.authToken = token;
				this.tokenExpiry = Date.now() + 55 * 60 * 1000;
				return token;
			}
		} catch (error) {
			console.error('Failed to get auth token:', error);
		}

		return null;
	}

	private static async fetchBinCollections(): Promise<BinCollectionRaw[]> {
		const config = await getBinCollectionsConfig();
		const uprn = config.uprn;
		const apiEndpoint = config.apiEndpoint;

		if (!uprn) {
			throw new Error('Missing UPRN in configuration');
		}

		const token = await this.getAuthToken(apiEndpoint);
		if (!token) {
			throw new Error('Authentication failed');
		}

		// Default to Fife Council endpoint if not configured
		const endpoint = apiEndpoint
			? `${apiEndpoint}/api/custom?action=powersuite_bin_calendar_collections&actionedby=bin_calendar&loadform=true&access=citizen&locale=en`
			: 'https://www.fife.gov.uk/api/custom?action=powersuite_bin_calendar_collections&actionedby=bin_calendar&loadform=true&access=citizen&locale=en';

		const response = await fetch(endpoint, {
			method: 'POST',
			headers: {
				Authorization: token,
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({
				name: 'bin_calendar',
				data: { uprn }
			})
		});

		if (!response.ok) {
			throw new Error(`HTTP error! status: ${response.status}`);
		}

		const data: BinApiResponse = await response.json();
		return (data.data?.tab_collections as BinCollectionRaw[]) || [];
	}

	static async getNextBinCollection(): Promise<{ date: string; bins: string[] } | null> {
		try {
			let collections: BinCollectionRaw[] = [];

			// Check cache first
			if (this.cache && Date.now() < this.cache.expiry) {
				collections = this.cache.collections;
			} else {
				// Fetch fresh data if cache is expired or doesn't exist
				collections = await this.fetchBinCollections();

				// Cache the results
				this.cache = {
					collections,
					expiry: Date.now() + TIMING_STRATEGIES.INFREQUENT.interval
				};
			}

			if (collections.length === 0) {
				return null;
			}

			const firstDate = collections[0].date;
			const firstDateObj = dayjs(firstDate);
			const today = dayjs();
			const tomorrow = today.add(1, 'day');

			// Only show if collection is today or tomorrow
			const isToday = firstDateObj.isSame(today, 'day');
			const isTomorrow = firstDateObj.isSame(tomorrow, 'day');

			if (!isToday && !isTomorrow) {
				return null; // Don't show if it's more than a day away
			}

			const nextBins = collections.filter((bin) => bin.date === firstDate).map((bin) => bin.colour);

			return { date: firstDate, bins: nextBins };
		} catch (error) {
			console.error('Failed to fetch bin collections:', error);
			return null;
		}
	}
}

export const GET: RequestHandler = async () => {
	try {
		const binData = await BinService.getNextBinCollection();
		return json(binData);
	} catch (error) {
		console.error('Error in bin collections API:', error);
		return json({ error: 'Failed to fetch bin data' }, { status: 500 });
	}
};
