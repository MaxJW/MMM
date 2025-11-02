import type { WeatherData, MeteoAlert } from './types';
import { TIMING_STRATEGIES } from '$lib/core/timing';
import { getCached, setCache } from '$lib/core/utils';
import type { CacheEntry } from '$lib/core/utils';
import { XMLParser } from 'fast-xml-parser';

interface WeatherConfig {
	apiKey?: string;
	latitude?: string;
	longitude?: string;
	alertsCountry?: string;
	alertsProvince?: string;
}

let weatherCache: CacheEntry<WeatherData> | null = null;
let alertsCache: CacheEntry<MeteoAlert[]> | null = null;

async function fetchWeather(config: WeatherConfig): Promise<WeatherData> {
	const { apiKey, latitude, longitude } = config;

	if (!apiKey) throw new Error('Missing Pirate Weather API key');
	if (!latitude || !longitude) throw new Error('Missing latitude or longitude');

	const res = await fetch(
		`https://api.pirateweather.net/forecast/${apiKey}/${latitude},${longitude}?units=si`
	);

	if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);

	const data = await res.json();

	const current = {
		tempC: Math.round(data.currently.temperature),
		condition: data.currently.summary,
		icon: data.currently.icon
	};

	const forecast = data.daily.data.slice(0, 3).map((d: any) => ({
		day: ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'][new Date(d.time * 1000).getDay()],
		icon: d.icon,
		hi: Math.round(d.temperatureHigh),
		lo: Math.round(d.temperatureLow)
	}));

	return { current, forecast };
}

async function fetchAlerts(config: WeatherConfig): Promise<MeteoAlert[]> {
	try {
		const country = config.alertsCountry || 'united-kingdom';
		const province = config.alertsProvince || '';
		const endpoint = `https://feeds.meteoalarm.org/feeds/meteoalarm-legacy-atom-${country}`;

		const res = await fetch(endpoint);
		if (!res.ok) throw new Error(`HTTP error: ${res.status}`);

		const xml = await res.text();
		const parser = new XMLParser({ ignoreAttributes: false, attributeNamePrefix: '' });
		const data = parser.parse(xml);

		if (!data?.feed?.entry) {
			console.warn('No feed entries found in MeteoAlarm response');
			return [];
		}

		const entries = Array.isArray(data.feed.entry) ? data.feed.entry : [data.feed.entry];

		const alerts: MeteoAlert[] = entries
			.filter((entry: Record<string, unknown>) => {
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
				if (!alert.end || !alert.area) return false;

				try {
					const endDate = new Date(alert.end);
					if (isNaN(endDate.getTime())) return false;

					if (province && !alert.area.includes(province)) return false;
					return endDate > new Date();
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

export async function GET(config: WeatherConfig, request?: Request): Promise<{
	weather?: WeatherData;
	alerts?: MeteoAlert[];
	error?: string;
}> {
	try {
		const url = request ? new URL(request.url) : null;
		const endpoint = url?.searchParams.get('endpoint') || 'forecast';

		if (endpoint === 'forecast') {
			// Check cache
			const cached = getCached(weatherCache);
			if (cached) {
				return { weather: cached };
			}

			const weather = await fetchWeather(config);
			weatherCache = setCache(weatherCache, weather, Date.now() + TIMING_STRATEGIES.STANDARD.interval);
			return { weather };
		} else if (endpoint === 'alerts') {
			// Check cache
			const cached = getCached(alertsCache);
			if (cached) {
				return { alerts: cached };
			}

			const alerts = await fetchAlerts(config);
			alertsCache = setCache(alertsCache, alerts, Date.now() + TIMING_STRATEGIES.STANDARD.interval);
			return { alerts };
		}

		return { error: 'Invalid endpoint' };
	} catch (error) {
		console.error('Weather API error:', error);
		return {
			error: error instanceof Error ? error.message : 'Failed to fetch weather data'
		};
	}
}
