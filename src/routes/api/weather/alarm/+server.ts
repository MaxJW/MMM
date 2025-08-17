import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { XMLParser } from 'fast-xml-parser';
import type { MeteoAlert } from '$lib/types/weather';
import { TIMING_STRATEGIES } from '$lib/types/util';

const COUNTRY = 'united-kingdom';
const ENDPOINT = `https://feeds.meteoalarm.org/feeds/meteoalarm-legacy-atom-${COUNTRY}`;
const PROVINCE = 'Central, Tayside & Fife';

class WeatherAlarmService {
	private static cache: { alerts: MeteoAlert[]; expiry: number } | null = null;

	private static async fetchAlerts(): Promise<MeteoAlert[]> {
		try {
			const res = await fetch(ENDPOINT);
			if (!res.ok) throw new Error(`HTTP error: ${res.status}`);

			const xml = await res.text();
			const parser = new XMLParser({ ignoreAttributes: false, attributeNamePrefix: '' });
			const data = parser.parse(xml);

			// Check if feed and entries exist
			if (!data?.feed?.entry) {
				console.warn('No feed entries found in MeteoAlarm response');
				return [];
			}

			const entries = Array.isArray(data.feed.entry) ? data.feed.entry : [data.feed.entry];

			const alerts: MeteoAlert[] = entries
				.filter((entry: Record<string, unknown>) => {
					// Filter out entries with missing required fields
					return (
						entry &&
						entry.title &&
						entry['cap:areaDesc'] &&
						entry['cap:event'] &&
						entry['cap:severity'] &&
						entry['cap:onset'] &&
						entry['cap:expires']
					);
				})
				.map((entry: Record<string, unknown>) => ({
					title: (entry.title as string) || 'Unknown Alert',
					description: (entry['cap:description'] as string) || '',
					area: (entry['cap:areaDesc'] as string) || '',
					event: (entry['cap:event'] as string) || 'Unknown Event',
					severity: (entry['cap:severity'] as string) || 'unknown',
					start: (entry['cap:onset'] as string) || '',
					end: (entry['cap:expires'] as string) || '',
					instruction: (entry['cap:instruction'] as string) || ''
				}))
				.filter((alert: MeteoAlert) => {
					// Additional validation
					if (!alert.end || !alert.area) return false;

					try {
						// Check if alert is not expired
						const endDate = new Date(alert.end);
						if (isNaN(endDate.getTime())) return false;

						// Check if alert is for our province
						return endDate > new Date() && alert.area.includes(PROVINCE);
					} catch (error) {
						console.warn('Error processing alert:', error, alert);
						return false;
					}
				});

			return alerts;
		} catch (error) {
			console.error('Error fetching MeteoAlarm alerts:', error);
			return [];
		}
	}

	static async getAlerts(): Promise<{ alerts: MeteoAlert[] }> {
		if (this.cache && Date.now() < this.cache.expiry) {
			return { alerts: this.cache.alerts };
		}

		const alerts = await this.fetchAlerts();
		this.cache = { alerts, expiry: Date.now() + TIMING_STRATEGIES.STANDARD.interval };
		return { alerts };
	}
}

export const GET: RequestHandler = async () => {
	try {
		const data = await WeatherAlarmService.getAlerts();
		return json(data);
	} catch (err) {
		console.error('Failed to fetch MeteoAlarm alerts:', err);
		return json({ alerts: [], error: 'Failed to fetch alerts' }, { status: 500 });
	}
};
