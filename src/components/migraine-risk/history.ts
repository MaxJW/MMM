import { readFile, writeFile, mkdir } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';

export interface MigraineReading {
	t: number;
	pressure_hPa: number;
	tempC: number;
}

interface HistoryFile {
	readings: MigraineReading[];
}

const HISTORY_MS = 48 * 60 * 60 * 1000;

export function getHistoryPath(): string {
	return join(process.cwd(), 'data', 'migraine-risk-history.json');
}

function prune(readings: MigraineReading[], now: number): MigraineReading[] {
	const cutoff = now - HISTORY_MS;
	return readings.filter((r) => r.t >= cutoff);
}

export async function loadHistory(): Promise<MigraineReading[]> {
	const path = getHistoryPath();
	if (!existsSync(path)) return [];
	try {
		const raw = await readFile(path, 'utf8');
		const parsed = JSON.parse(raw) as HistoryFile;
		return Array.isArray(parsed.readings) ? parsed.readings : [];
	} catch {
		return [];
	}
}

export async function appendReading(reading: MigraineReading): Promise<void> {
	const dir = join(process.cwd(), 'data');
	if (!existsSync(dir)) {
		await mkdir(dir, { recursive: true });
	}
	const now = reading.t;
	let readings = await loadHistory();
	readings.push(reading);
	readings = prune(readings, now);
	await writeFile(
		getHistoryPath(),
		JSON.stringify({ readings } satisfies HistoryFile, null, 2),
		'utf8'
	);
}

/** Peak pressure in [now - windowHours, now] from history (exclusive of current moment for past only). Includes comparing peak to `currentPressure`. */
export function pressureDropFromPeak(
	readings: MigraineReading[],
	currentPressure: number,
	nowMs: number,
	windowHours: number
): number {
	const windowMs = windowHours * 60 * 60 * 1000;
	const start = nowMs - windowMs;
	const inWindow = readings.filter((r) => r.t >= start && r.t <= nowMs);
	let peak = currentPressure;
	for (const r of inWindow) {
		if (r.pressure_hPa > 900 && r.pressure_hPa > peak) peak = r.pressure_hPa;
	}
	if (currentPressure > 900 && currentPressure > peak) peak = currentPressure;
	const drop = peak - currentPressure;
	return Math.max(0, Math.round(drop * 10) / 10);
}

/** Absolute °C change vs reading nearest to (now - 6h). */
export function temperatureChange6h(
	readings: MigraineReading[],
	currentTemp: number,
	nowMs: number
): number {
	const target = nowMs - 6 * 60 * 60 * 1000;
	if (readings.length === 0) return 0;

	let best = readings[0];
	let bestDt = Math.abs(best.t - target);
	for (const r of readings) {
		const dt = Math.abs(r.t - target);
		if (dt < bestDt) {
			bestDt = dt;
			best = r;
		}
	}
	return Math.round(Math.abs(currentTemp - best.tempC) * 10) / 10;
}
