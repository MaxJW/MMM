export type WeatherIcon =
	| 'clear-day'
	| 'clear-night'
	| 'rain'
	| 'snow'
	| 'sleet'
	| 'wind'
	| 'fog'
	| 'cloudy'
	| 'partly-cloudy-day'
	| 'partly-cloudy-night'
	| 'thunderstorm'
	| 'hail'
	| 'mixed'
	| 'none'; // For cases where the API returns no icon

export interface WeatherCurrent {
	tempC: number;
	condition: string;
	icon: WeatherIcon;
}

export interface WeatherForecast {
	day: string;
	icon: WeatherIcon;
	hi: number;
	lo: number;
}

export interface WeatherData {
	current: WeatherCurrent;
	forecast: WeatherForecast[];
}

export interface MeteoAlert {
	title: string;
	description: string;
	severity: string; // e.g., "green", "yellow", "amber", "red"
	area: string;
	start: string;
	end: string;
	instruction?: string;
	event: string;
}
