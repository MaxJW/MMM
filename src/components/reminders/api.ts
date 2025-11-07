import dayjs from 'dayjs';
import type { BinApiResponse } from './types';
import { TIMING_STRATEGIES } from '$lib/core/timing';
import { getCached, setCache } from '$lib/core/utils';
import type { CacheEntry } from '$lib/core/utils';

let cache: CacheEntry<BinCollectionRaw[]> | null = null;
let authToken: string | null = null;
let tokenExpiry: number = 0;

interface BinCollectionRaw {
	date: string;
	colour: string;
}

interface RemindersConfig {
	uprn?: string;
	apiEndpoint?: string;
}

import type { BinCollection } from './types';

async function getAuthToken(apiEndpoint?: string): Promise<string | null> {
	if (authToken && Date.now() < tokenExpiry) {
		return authToken;
	}

	const authUrl = apiEndpoint
		? `${apiEndpoint}/api/citizen?preview=false&locale=en`
		: 'https://www.fife.gov.uk/api/citizen?preview=false&locale=en';

	try {
		const response = await fetch(authUrl);
		const token = response.headers.get('Authorization');

		if (token) {
			authToken = token;
			tokenExpiry = Date.now() + 55 * 60 * 1000;
			return token;
		}
	} catch (error) {
		console.error('Failed to get auth token:', error);
	}

	return null;
}

async function fetchBinCollections(config: RemindersConfig): Promise<BinCollectionRaw[]> {
	const uprn = config.uprn;
	const apiEndpoint = config.apiEndpoint;

	if (!uprn) {
		throw new Error('Missing UPRN in configuration');
	}

	const token = await getAuthToken(apiEndpoint);
	if (!token) {
		throw new Error('Authentication failed');
	}

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

export async function GET(
	config: RemindersConfig
): Promise<BinCollection | null | { error: string }> {
	try {
		let collections: BinCollectionRaw[] = [];

		// Check cache first
		const cached = getCached(cache);
		if (cached) {
			collections = cached;
		} else {
			// Fetch fresh data
			collections = await fetchBinCollections(config);
			cache = setCache(cache, collections, Date.now() + TIMING_STRATEGIES.INFREQUENT.interval);
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
		console.error('Bin collections API error:', error);
		return { error: error instanceof Error ? error.message : 'Failed to fetch bin collections' };
	}
}

function formatDate(dateString: string): string {
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

export { formatDate };
