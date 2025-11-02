import type { SystemStats } from './types';
import { TIMING_STRATEGIES } from '$lib/core/timing';
import { getCached, setCache } from '$lib/core/utils';
import type { CacheEntry } from '$lib/core/utils';
import { exec } from 'child_process';
import { promisify } from 'util';
import { readFile } from 'fs/promises';
import { dev } from '$app/environment';

const execAsync = promisify(exec);

let cache: CacheEntry<SystemStats> | null = null;

async function getCpuUsage(): Promise<number> {
	// Try multiple methods to get CPU usage, with fallbacks
	// Method 1: Use vmstat if available (most reliable single-reading method)
	try {
		const { stdout } = await execAsync("vmstat 1 2 | tail -1 | awk '{print 100 - $15}'");
		const value = parseFloat(stdout.trim());
		if (!isNaN(value) && value >= 0 && value <= 100) {
			return Math.round(value);
		}
	} catch {
		// Continue to next method
	}

	// Method 2: Use top with better parsing for different formats
	try {
		const { stdout } = await execAsync(
			'top -bn1 | grep -E \'%Cpu|%cpu|Cpu\\(s\\)\' | head -1 | awk \'{for(i=1;i<=NF;i++){if($i ~ /%sy|%us|%ni|%wa|%hi|%si|%st|%id/){split($i,a,"%");if(a[1]~/id/){idle=a[1];gsub(/[^0-9.]/,"",idle);print 100-idle}}}}\' | head -1'
		);
		const value = parseFloat(stdout.trim());
		if (!isNaN(value) && value >= 0 && value <= 100) {
			return Math.round(value);
		}
	} catch {
		// Continue to next method
	}

	// Method 3: Parse top output for idle percentage
	try {
		const { stdout } = await execAsync(
			"top -bn1 | grep -E '%Cpu|%cpu|Cpu\\(s\\)' | head -1 | awk -F'id,' '{print $2}' | awk '{print 100-$1}' | sed 's/%//'"
		);
		const value = parseFloat(stdout.trim());
		if (!isNaN(value) && value >= 0 && value <= 100) {
			return Math.round(value);
		}
	} catch {
		// Continue to next method
	}

	// Method 4: Use /proc/stat with approximation (less accurate but works)
	try {
		const stat = await readFile('/proc/stat', 'utf8');
		const lines = stat.split('\n');
		const cpuLine = lines.find((line) => line.startsWith('cpu '));

		if (cpuLine) {
			const values = cpuLine.split(/\s+/).slice(1).map(Number);
			// Calculate non-idle percentage (approximation from single reading)
			// This won't be perfectly accurate but gives a reasonable estimate
			const total = values.reduce((a, b) => a + b, 0);
			const idle = values[3] + (values[4] || 0); // idle + iowait
			if (total > 0) {
				const usage = ((total - idle) / total) * 100;
				if (!isNaN(usage) && usage >= 0 && usage <= 100) {
					return Math.round(usage);
				}
			}
		}
	} catch {
		// Return 0 if all methods fail
	}

	return 0;
}

async function getMemoryUsage(): Promise<number> {
	try {
		const { stdout } = await execAsync('free | grep Mem | awk \'{printf "%.1f", $3/$2 * 100.0}\'');
		return Math.round(parseFloat(stdout.trim()));
	} catch {
		return 0;
	}
}

async function getDiskUsage(): Promise<number> {
	try {
		const { stdout } = await execAsync("df / | tail -1 | awk '{print $5}' | sed 's/%//'");
		return parseInt(stdout.trim());
	} catch {
		return 0;
	}
}

async function getTemperature(): Promise<number> {
	try {
		const commands = [
			'cat /sys/class/thermal/thermal_zone0/temp',
			'vcgencmd measure_temp',
			'cat /proc/acpi/thermal_zone/THM0/temperature'
		];

		for (const cmd of commands) {
			try {
				const { stdout } = await execAsync(cmd);
				let temp = stdout.trim();

				if (temp.includes('temp=')) {
					temp = temp.replace('temp=', '').replace("'C", '');
					return Math.round(parseFloat(temp));
				} else if (temp.length > 3) {
					return Math.round(parseInt(temp) / 1000);
				} else {
					return parseInt(temp);
				}
			} catch {
				continue;
			}
		}
		return 0;
	} catch {
		return 0;
	}
}

async function fetchSystemStats(): Promise<SystemStats> {
	// Return placeholder data in development mode
	if (dev) {
		return {
			cpu: Math.floor(Math.random() * 50) + 10,
			memory: Math.floor(Math.random() * 40) + 30,
			disk: Math.floor(Math.random() * 30) + 50,
			tempC: Math.floor(Math.random() * 20) + 35
		};
	}

	const [cpu, memory, disk, tempC] = await Promise.all([
		getCpuUsage(),
		getMemoryUsage(),
		getDiskUsage(),
		getTemperature()
	]);

	return { cpu, memory, disk, tempC };
}

// eslint-disable-next-line @typescript-eslint/no-unused-vars
export async function GET(_config?: unknown): Promise<SystemStats | { error: string }> {
	try {
		// Check cache
		const cached = getCached(cache);
		if (cached) {
			return cached;
		}

		const stats = await fetchSystemStats();
		cache = setCache(cache, stats, Date.now() + TIMING_STRATEGIES.FREQUENT.interval);
		return stats;
	} catch (error) {
		console.error('System stats API error:', error);
		return {
			error: error instanceof Error ? error.message : 'Failed to fetch system stats'
		};
	}
}
