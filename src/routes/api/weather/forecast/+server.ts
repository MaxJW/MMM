import { json } from '@sveltejs/kit';
import type { RequestHandler } from '../$types';
import type { WeatherData } from '$lib/types/weather';
import { PIRATE_WEATHER_KEY } from '$env/static/private';
import { TIMING_STRATEGIES } from '$lib/types/util';

class WeatherService {
	private static cache: { data: WeatherData; expiry: number } | null = null;

	private static async fetchWeather(): Promise<WeatherData> {
		const latitude = '56.14087978381211';
		const longitude = '-3.1256827712059025';
		const apiKey = PIRATE_WEATHER_KEY;

		if (!apiKey) throw new Error('Missing Pirate Weather API key');

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

	static async getWeather(): Promise<WeatherData> {
		if (this.cache && Date.now() < this.cache.expiry) {
			return this.cache.data;
		}
		const data = await this.fetchWeather();
		this.cache = { data, expiry: Date.now() + TIMING_STRATEGIES.STANDARD.interval };
		return data;
	}
}

export const GET: RequestHandler = async () => {
	try {
		const weather = await WeatherService.getWeather();
		return json(weather);
	} catch (error) {
		console.error('Failed to fetch weather:', error);
		return json({ error: 'Failed to fetch weather data' }, { status: 500 });
	}
};
