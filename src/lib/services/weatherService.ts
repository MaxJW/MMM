import type { WeatherData, MeteoAlert } from '$lib/types/weather';

interface MeteoAlarmApiResponse {
	alerts: MeteoAlert[];
}

export class WeatherService {
	static async getWeather(): Promise<WeatherData | null> {
		try {
			const res = await fetch('/api/weather/forecast');
			if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);

			const data = await res.json();
			if (!data || data.error) return null;

			return data as WeatherData;
		} catch (error) {
			console.error('Failed to fetch weather:', error);
			return null;
		}
	}

	static async getAlerts(): Promise<MeteoAlert[]> {
		try {
			const response = await fetch('/api/weather/alarm');

			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			const data: MeteoAlarmApiResponse = await response.json();
			if (!data || !Array.isArray(data.alerts)) return [];

			return data.alerts as MeteoAlert[];
		} catch (err) {
			console.error('Failed to fetch MeteoAlarm alerts:', err);
			return [];
		}
	}
}
