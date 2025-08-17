import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { XMLParser } from 'fast-xml-parser';
import type { MeteoAlert } from '$lib/types/weather';
import { THIRTY_MIN } from '$lib/types/util';

const COUNTRY = 'united-kingdom';
const ENDPOINT = `https://feeds.meteoalarm.org/feeds/meteoalarm-legacy-atom-${COUNTRY}`;
const PROVINCE = 'Central, Tayside & Fife';

interface CachedData {
	alerts: MeteoAlert[];
	expiry: number;
}

let cache: CachedData | null = null;

export const GET: RequestHandler = async () => {
	if (cache && Date.now() < cache.expiry) return json({ alerts: cache.alerts });

	try {
		const res = await fetch(ENDPOINT);
		if (!res.ok) throw new Error(`HTTP error: ${res.status}`);

		const xml = await res.text();
		const parser = new XMLParser({ ignoreAttributes: false, attributeNamePrefix: '' });
		const data = parser.parse(xml);

		const entries = Array.isArray(data.feed.entry) ? data.feed.entry : [data.feed.entry];

		const alerts: MeteoAlert[] = entries
			.map((entry: any) => ({
				title: entry.title,
				area: entry['cap:areaDesc'],
				event: entry['cap:event'],
				severity: entry['cap:severity'],
				onset: entry['cap:onset'],
				expires: entry['cap:expires'],
				link: Array.isArray(entry.link) ? entry.link[0].href : entry.link.href
			}))
			.filter(
				(a) =>
					new Date(a.expires) > new Date() && // not expired
					a.area.includes(PROVINCE) // only your province
			);

		cache = { alerts, expiry: Date.now() + THIRTY_MIN };
		return json({ alerts });
	} catch (err) {
		console.error('Failed to fetch MeteoAlarm alerts:', err);
		return json({ alerts: [], error: 'Failed to fetch alerts' }, { status: 500 });
	}
};
