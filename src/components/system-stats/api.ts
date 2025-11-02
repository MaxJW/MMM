import type { SystemStats } from './types';
import { TIMING_STRATEGIES } from '$lib/core/timing';
import { getCached, setCache } from '$lib/core/utils';
import type { CacheEntry } from '$lib/core/utils';
import { exec } from 'child_process';
import { promisify } from 'util';
import { dev } from '$app/environment';

const execAsync = promisify(exec);

let cache: CacheEntry<SystemStats> | null = null;

async function getCpuUsage(): Promise<number> {
	try {
		const { stdout } = await execAsync(
			"top -bn1 | grep 'Cpu(s)' | awk '{print $2}' | cut -d'%' -f1"
		);
		return Math.round(parseFloat(stdout.trim()));
	} catch {
		return 0;
	}
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

export async function GET(config: any): Promise<SystemStats | { error: string }> {
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
