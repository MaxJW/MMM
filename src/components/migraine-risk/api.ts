/**
 * Standalone migraine risk using Pirate Weather (same config as Weather component).
 * Scoring thresholds from migraine-risk-card (MIT) / HA sensor package.
 */

import { getComponentConfig } from '$lib/core/config';
import { fetchMeteoAlerts, fetchPirateWeatherJson } from '../weather/api';
import type { MeteoAlert } from '../weather/types';
import { appendReading, loadHistory, pressureDropFromPeak, temperatureChange6h } from './history';
import {
	computeFactorBreakdown,
	sumFactors,
	todayRiskLevel,
	tomorrowForecastScore,
	tomorrowRiskLevel,
	windMpsToKmh,
	type ThunderstormInputs
} from './scoring';
import type { MigraineRiskPayload } from './types';
import { MIGRAINE_MAX_SCORE } from './types';

interface PWCurrently {
	time?: number;
	pressure?: number;
	humidity?: number;
	temperature?: number;
	windSpeed?: number;
	uvIndex?: number;
	icon?: string;
	summary?: string;
}

interface PWDayBlock {
	temperatureHigh?: number;
	temperatureLow?: number;
	temperatureMax?: number;
	temperatureMin?: number;
	uvIndex?: number;
	precipProbability?: number;
	summary?: string;
	icon?: string;
}

interface PWResponse {
	currently?: PWCurrently;
	hourly?: { data?: PWCurrently[] };
	daily?: { data?: PWDayBlock[] };
}

function num(n: unknown, fallback = 0): number {
	if (typeof n === 'number' && !Number.isNaN(n)) return n;
	return fallback;
}

function pickDailyExtremes(d: PWDayBlock): { tmax: number; tmin: number } {
	const hi = d.temperatureHigh ?? d.temperatureMax;
	const lo = d.temperatureLow ?? d.temperatureMin;
	return { tmax: num(hi), tmin: num(lo) };
}

function buildAlertsText(alerts: MeteoAlert[]): string {
	return alerts.map((a) => `${a.event} ${a.title} ${a.description}`.toLowerCase()).join(' ');
}

function hasThunderInCurrentOrHourly(data: PWResponse): boolean {
	const cur = data.currently;
	if (cur?.icon === 'thunderstorm') return true;
	const s = (cur?.summary ?? '').toLowerCase();
	if (s.includes('thunder') || s.includes('thunderstorm')) return true;

	const hourly = data.hourly?.data ?? [];
	const nowSec = Math.floor(Date.now() / 1000);
	for (const h of hourly) {
		const t = num(h.time, nowSec);
		if (t < nowSec || t > nowSec + 18 * 3600) continue;
		if (h.icon === 'thunderstorm') return true;
		const hs = (h.summary ?? '').toLowerCase();
		if (hs.includes('thunder')) return true;
	}
	return false;
}

function buildPayload(
	data: PWResponse,
	alerts: MeteoAlert[],
	pressureDrop6h: number,
	pressureDrop24h: number,
	tempChange6h: number
): MigraineRiskPayload {
	const cur = data.currently ?? {};
	const humidityPct = num(cur.humidity);
	const tempC = num(cur.temperature);
	const windKmh = windMpsToKmh(num(cur.windSpeed));
	const uvIndex = num(cur.uvIndex);

	const fc = (cur.summary ?? '').toLowerCase();
	const alertsText = buildAlertsText(alerts);

	const thunderInput: ThunderstormInputs = {
		alertsText,
		forecastText: fc,
		hasThunderCondition: hasThunderInCurrentOrHourly(data)
	};

	const factors = computeFactorBreakdown({
		pressureDrop6h,
		pressureDrop24h,
		humidityPct,
		tempC,
		tempChange6h,
		windKmh,
		uvIndex,
		thunderstorm: thunderInput,
		airQuality: { aqi: 0, pm25: 0, pm10: 0, o3: 0 }
	});

	const score = Math.min(sumFactors(factors), MIGRAINE_MAX_SCORE);
	const level = todayRiskLevel(score);

	const daily = data.daily?.data ?? [];
	const d1 = daily.length > 1 ? daily[1] : null;
	let tomorrowScore: number | null = null;
	let tomorrowLevel: ReturnType<typeof tomorrowRiskLevel> | null = null;

	if (d1) {
		const { tmax, tmin } = pickDailyExtremes(d1);
		const hasTemps = (d1.temperatureHigh ?? d1.temperatureMax) != null;

		if (hasTemps) {
			const uvT = num(d1.uvIndex, uvIndex);
			const summary = (d1.summary ?? '').toLowerCase();
			const precip = num(d1.precipProbability, 0);
			tomorrowScore = tomorrowForecastScore({
				tmax,
				tmin,
				uv: uvT,
				forecastTextLower: summary,
				rainChancePct: Math.round(precip * 100)
			});
			if (tomorrowScore !== null) {
				tomorrowLevel = tomorrowRiskLevel(tomorrowScore);
			}
		}
	}

	const factorLabels = {
		pressure6h: 'Pressure (6h drop)',
		pressure24h: 'Pressure (24h drop)',
		humidity: 'Humidity',
		temperature: 'Temperature',
		temperatureChange: 'Temp. change (6h)',
		wind: 'Wind',
		uv: 'UV',
		thunderstorm: 'Thunderstorm',
		airQuality: 'Air quality'
	} as const;

	return {
		score,
		level,
		maxScore: MIGRAINE_MAX_SCORE,
		factors,
		factorLabels,
		tomorrow: {
			score: tomorrowScore,
			level: tomorrowLevel,
			available: tomorrowScore !== null
		},
		updatedAt: new Date().toISOString(),
		meta: {
			pressureDrop6h_hPa: pressureDrop6h,
			pressureDrop24h_hPa: pressureDrop24h,
			temperatureChange6h_C: tempChange6h,
			windKmh,
			humidityPct,
			tempC,
			uvIndex,
			airQualityAvailable: false
		}
	};
}

export async function GET(
	// Signature matches component API handler; config is unused (weather config is loaded in-handler).
	_config: Record<string, unknown>
): Promise<MigraineRiskPayload | { error: string }> {
	void _config;
	try {
		const weather = (await getComponentConfig('weather')) as Record<string, string | undefined>;

		if (!weather?.apiKey || !weather?.latitude || !weather?.longitude) {
			return {
				error:
					'Configure the Weather component with Pirate Weather API key, latitude, and longitude.'
			};
		}

		const raw = (await fetchPirateWeatherJson({
			apiKey: weather.apiKey,
			latitude: String(weather.latitude),
			longitude: String(weather.longitude),
			alertsCountry: weather.alertsCountry,
			alertsProvince: weather.alertsProvince
		})) as PWResponse;

		const alerts = await fetchMeteoAlerts({
			apiKey: weather.apiKey,
			latitude: String(weather.latitude),
			longitude: String(weather.longitude),
			alertsCountry: weather.alertsCountry,
			alertsProvince: weather.alertsProvince
		});

		const now = Date.now();
		const cur = raw.currently ?? {};
		const pressure = num(cur.pressure);
		const tempC = num(cur.temperature);

		const history = await loadHistory();
		const pressureDrop6h = pressure > 900 ? pressureDropFromPeak(history, pressure, now, 6) : 0;
		const pressureDrop24h = pressure > 900 ? pressureDropFromPeak(history, pressure, now, 24) : 0;
		const tempChange6h = temperatureChange6h(history, tempC, now);

		const payload = buildPayload(raw, alerts, pressureDrop6h, pressureDrop24h, tempChange6h);

		await appendReading({
			t: now,
			pressure_hPa: pressure > 900 ? pressure : 0,
			tempC
		});

		return payload;
	} catch (e) {
		console.error('migraine-risk API:', e);
		return { error: e instanceof Error ? e.message : 'Failed to load migraine risk data' };
	}
}
