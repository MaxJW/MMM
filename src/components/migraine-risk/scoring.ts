// Scoring logic ported from migraine-risk-card sensor-package (MIT).

import type { FactorBreakdown, RiskLevelLabel } from './types';

export function windMpsToKmh(mps: number): number {
	return mps * 3.6;
}

/** Points from 6 h pressure drop (hPa): YAML thresholds. */
export function pressureDrop6hPoints(dropHpa: number): number {
	const d6 = dropHpa;
	if (d6 >= 10) return 4;
	if (d6 >= 8) return 3;
	if (d6 >= 6) return 2;
	if (d6 >= 4) return 1;
	return 0;
}

/** Points from 24 h pressure drop (hPa). */
export function pressureDrop24hPoints(dropHpa: number): number {
	const d24 = dropHpa;
	if (d24 >= 14) return 3;
	if (d24 >= 10) return 2;
	if (d24 >= 6) return 1;
	return 0;
}

export function humidityFactor(h: number): number {
	if (h > 80) return 2;
	if (h < 30) return 1;
	return 0;
}

export function temperatureFactor(t: number): number {
	if (t > 30 || t < 5) return 2;
	return 0;
}

export function temperatureChangeFactor(deltaC: number): number {
	if (deltaC >= 8) return 2;
	if (deltaC >= 5) return 1;
	return 0;
}

/** Wind in km/h (HA package uses km/h). */
export function windFactor(windKmh: number): number {
	if (windKmh >= 50) return 2;
	if (windKmh >= 35) return 1;
	return 0;
}

export function uvFactor(uv: number): number {
	if (uv >= 8) return 2;
	if (uv >= 6) return 1;
	return 0;
}

/** WAQI-style AQI + optional PM/O3 nudge; capped at 3. */
export function airQualityFactor(aqi: number, pm25: number, pm10: number, o3: number): number {
	const base = aqi <= 50 ? 0 : aqi <= 100 ? 1 : aqi <= 150 ? 2 : 3;
	let nudge = 0;
	if (pm25 >= 25) nudge += 1;
	if (pm10 >= 50) nudge += 1;
	if (o3 >= 120) nudge += 1;
	return Math.min(base + nudge, 3);
}

export interface ThunderstormInputs {
	/** Lowercased text from weather warnings / CAP alerts. */
	alertsText: string;
	/** Lowercased forecast text (e.g. summary). */
	forecastText: string;
	/** True if current/hourly icon or condition implies thunder. */
	hasThunderCondition: boolean;
}

export function thunderstormFactor(input: ThunderstormInputs): number {
	const warningsText = input.alertsText;
	const forecastText = input.forecastText;

	const warningThunder = warningsText.includes('thunderstorm');
	const forecastThunder =
		forecastText.includes('thunderstorm') ||
		forecastText.includes('thunder') ||
		input.hasThunderCondition;

	if (warningThunder) return 2;
	if (forecastThunder) return 1;
	return 0;
}

export function todayRiskLevel(score: number): RiskLevelLabel {
	if (score >= 12) return 'Very High';
	if (score >= 8) return 'High';
	if (score >= 4) return 'Moderate';
	return 'Low';
}

export function tomorrowRiskLevel(score: number): RiskLevelLabel {
	if (score >= 8) return 'Very High';
	if (score >= 5) return 'High';
	if (score >= 3) return 'Moderate';
	return 'Low';
}

export function computeFactorBreakdown(args: {
	pressureDrop6h: number;
	pressureDrop24h: number;
	humidityPct: number;
	tempC: number;
	tempChange6h: number;
	windKmh: number;
	uvIndex: number;
	thunderstorm: ThunderstormInputs;
	airQuality: { aqi: number; pm25: number; pm10: number; o3: number };
}): FactorBreakdown {
	return {
		pressure6h: pressureDrop6hPoints(args.pressureDrop6h),
		pressure24h: pressureDrop24hPoints(args.pressureDrop24h),
		humidity: humidityFactor(args.humidityPct),
		temperature: temperatureFactor(args.tempC),
		temperatureChange: temperatureChangeFactor(args.tempChange6h),
		wind: windFactor(args.windKmh),
		uv: uvFactor(args.uvIndex),
		thunderstorm: thunderstormFactor(args.thunderstorm),
		airQuality: airQualityFactor(
			args.airQuality.aqi,
			args.airQuality.pm25,
			args.airQuality.pm10,
			args.airQuality.o3
		)
	};
}

export function sumFactors(f: FactorBreakdown): number {
	return (
		f.pressure6h +
		f.pressure24h +
		f.humidity +
		f.temperature +
		f.temperatureChange +
		f.wind +
		f.uv +
		f.thunderstorm +
		f.airQuality
	);
}

export interface TomorrowForecastInput {
	tmax: number;
	tmin: number;
	uv: number;
	forecastTextLower: string;
	rainChancePct: number;
}

/**
 * Port of sensor `migraine_risk_forecast_tomorrow` state (YAML ~676–704).
 * Returns null if tomorrow max temp is unavailable (invalid / missing daily[1]).
 */
export function tomorrowForecastScore(input: TomorrowForecastInput | null): number | null {
	if (!input) return null;

	const { tmax, tmin, uv } = input;
	const fc_text = input.forecastTextLower;

	const temp_pts = tmax > 30 || tmin < 5 ? 2 : tmax > 28 || tmin < 8 ? 1 : 0;
	const temp_range = tmax - tmin;
	const swing_pts = temp_range >= 15 ? 2 : temp_range >= 10 ? 1 : 0;
	const uv_pts = uv >= 8 ? 2 : uv >= 6 ? 1 : 0;

	let wind_pts = 0;
	if (fc_text.includes('damaging winds') || fc_text.includes('destructive winds')) {
		wind_pts = 2;
	} else if (
		fc_text.includes('strong winds') ||
		fc_text.includes('45 to') ||
		fc_text.includes('50 to') ||
		fc_text.includes('55 to') ||
		fc_text.includes('60 to')
	) {
		wind_pts = 2;
	} else if (fc_text.includes('35 to') || fc_text.includes('40 to')) {
		wind_pts = 1;
	}

	const thunder_pts = fc_text.includes('thunderstorm') || fc_text.includes('thunder') ? 2 : 0;
	const hum_pts = input.rainChancePct >= 80 ? 1 : 0;

	return temp_pts + swing_pts + uv_pts + wind_pts + thunder_pts + hum_pts;
}
