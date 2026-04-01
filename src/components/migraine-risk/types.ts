/** Scoring thresholds ported from migraine-risk-card (MIT) / HA sensor package. */

export const MIGRAINE_MAX_SCORE = 22;

export type RiskLevelLabel = 'Low' | 'Moderate' | 'High' | 'Very High';

export interface FactorBreakdown {
	pressure6h: number;
	pressure24h: number;
	humidity: number;
	temperature: number;
	temperatureChange: number;
	wind: number;
	uv: number;
	thunderstorm: number;
	airQuality: number;
}

export interface MigraineRiskPayload {
	score: number;
	level: RiskLevelLabel;
	maxScore: typeof MIGRAINE_MAX_SCORE;
	factors: FactorBreakdown;
	factorLabels: Record<keyof FactorBreakdown, string>;
	tomorrow: {
		score: number | null;
		level: RiskLevelLabel | null;
		available: boolean;
	};
	updatedAt: string;
	meta: {
		pressureDrop6h_hPa: number;
		pressureDrop24h_hPa: number;
		temperatureChange6h_C: number;
		windKmh: number;
		humidityPct: number;
		tempC: number;
		uvIndex: number;
		airQualityAvailable: boolean;
	};
}
