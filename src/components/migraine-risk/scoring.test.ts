import { describe, expect, test } from 'bun:test';
import {
	pressureDrop6hPoints,
	pressureDrop24hPoints,
	humidityFactor,
	temperatureFactor,
	temperatureChangeFactor,
	windFactor,
	uvFactor,
	airQualityFactor,
	thunderstormFactor,
	todayRiskLevel,
	tomorrowForecastScore,
	tomorrowRiskLevel,
	windMpsToKmh,
	sumFactors,
	computeFactorBreakdown
} from './scoring';

describe('windMpsToKmh', () => {
	test('converts SI to km/h', () => {
		expect(windMpsToKmh(10)).toBe(36);
		expect(windMpsToKmh(0)).toBe(0);
	});
});

describe('pressureDrop6hPoints', () => {
	test('matches YAML thresholds', () => {
		expect(pressureDrop6hPoints(0)).toBe(0);
		expect(pressureDrop6hPoints(3.9)).toBe(0);
		expect(pressureDrop6hPoints(4)).toBe(1);
		expect(pressureDrop6hPoints(7.9)).toBe(2);
		expect(pressureDrop6hPoints(8)).toBe(3);
		expect(pressureDrop6hPoints(9.9)).toBe(3);
		expect(pressureDrop6hPoints(10)).toBe(4);
	});
});

describe('pressureDrop24hPoints', () => {
	test('matches YAML thresholds', () => {
		expect(pressureDrop24hPoints(0)).toBe(0);
		expect(pressureDrop24hPoints(5.9)).toBe(0);
		expect(pressureDrop24hPoints(6)).toBe(1);
		expect(pressureDrop24hPoints(13.9)).toBe(2);
		expect(pressureDrop24hPoints(14)).toBe(3);
	});
});

describe('humidityFactor', () => {
	test('edges', () => {
		expect(humidityFactor(45)).toBe(0);
		expect(humidityFactor(29.9)).toBe(1);
		expect(humidityFactor(80.1)).toBe(2);
	});
});

describe('temperatureFactor', () => {
	test('extremes', () => {
		expect(temperatureFactor(20)).toBe(0);
		expect(temperatureFactor(30.1)).toBe(2);
		expect(temperatureFactor(4.9)).toBe(2);
	});
});

describe('temperatureChangeFactor', () => {
	test('bands', () => {
		expect(temperatureChangeFactor(4)).toBe(0);
		expect(temperatureChangeFactor(5)).toBe(1);
		expect(temperatureChangeFactor(8)).toBe(2);
	});
});

describe('windFactor', () => {
	test('km/h', () => {
		expect(windFactor(34)).toBe(0);
		expect(windFactor(35)).toBe(1);
		expect(windFactor(49)).toBe(1);
		expect(windFactor(50)).toBe(2);
	});
});

describe('uvFactor', () => {
	test('bands', () => {
		expect(uvFactor(5)).toBe(0);
		expect(uvFactor(6)).toBe(1);
		expect(uvFactor(8)).toBe(2);
	});
});

describe('airQualityFactor', () => {
	test('base and cap', () => {
		expect(airQualityFactor(40, 0, 0, 0)).toBe(0);
		expect(airQualityFactor(75, 0, 0, 0)).toBe(1);
		expect(airQualityFactor(120, 0, 0, 0)).toBe(2);
		expect(airQualityFactor(160, 0, 0, 0)).toBe(3);
		expect(airQualityFactor(160, 30, 60, 130)).toBe(3);
	});
});

describe('thunderstormFactor', () => {
	test('warning beats forecast', () => {
		expect(
			thunderstormFactor({
				alertsText: 'severe thunderstorm warning',
				forecastText: '',
				hasThunderCondition: false
			})
		).toBe(2);
		expect(
			thunderstormFactor({
				alertsText: '',
				forecastText: 'possible thunder later',
				hasThunderCondition: false
			})
		).toBe(1);
		expect(
			thunderstormFactor({
				alertsText: '',
				forecastText: '',
				hasThunderCondition: true
			})
		).toBe(1);
	});
});

describe('risk levels', () => {
	test('today', () => {
		expect(todayRiskLevel(0)).toBe('Low');
		expect(todayRiskLevel(4)).toBe('Moderate');
		expect(todayRiskLevel(8)).toBe('High');
		expect(todayRiskLevel(12)).toBe('Very High');
	});
	test('tomorrow bands', () => {
		expect(tomorrowRiskLevel(2)).toBe('Low');
		expect(tomorrowRiskLevel(3)).toBe('Moderate');
		expect(tomorrowRiskLevel(5)).toBe('High');
		expect(tomorrowRiskLevel(8)).toBe('Very High');
	});
});

describe('tomorrowForecastScore', () => {
	test('returns null when input null', () => {
		expect(tomorrowForecastScore(null)).toBeNull();
	});
	test('combined scenario', () => {
		const s = tomorrowForecastScore({
			tmax: 32,
			tmin: 10,
			uv: 7,
			forecastTextLower: 'strong winds and thunderstorm possible',
			rainChancePct: 85
		});
		expect(s).toBeGreaterThan(0);
	});
});

describe('computeFactorBreakdown + sumFactors', () => {
	test('all zeros', () => {
		const f = computeFactorBreakdown({
			pressureDrop6h: 0,
			pressureDrop24h: 0,
			humidityPct: 50,
			tempC: 20,
			tempChange6h: 0,
			windKmh: 10,
			uvIndex: 0,
			thunderstorm: { alertsText: '', forecastText: '', hasThunderCondition: false },
			airQuality: { aqi: 0, pm25: 0, pm10: 0, o3: 0 }
		});
		expect(sumFactors(f)).toBe(0);
	});
});
