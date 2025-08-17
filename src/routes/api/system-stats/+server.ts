import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { exec } from 'child_process';
import { promisify } from 'util';
import { dev } from '$app/environment';
import type { SystemStats } from '$lib/types/system-stats';
import { TIMING_STRATEGIES } from '$lib/types/util';

const execAsync = promisify(exec);

class SystemStatsService {
	private static cache: { data: SystemStats; expiry: number } | null = null;

	private static async fetchSystemStats(): Promise<SystemStats> {
		// Return placeholder data in development mode
		if (dev) {
			return {
				cpu: Math.floor(Math.random() * 50) + 10, // Random between 10-60%
				memory: Math.floor(Math.random() * 40) + 30, // Random between 30-70%
				disk: Math.floor(Math.random() * 30) + 50, // Random between 50-80%
				tempC: Math.floor(Math.random() * 20) + 35 // Random between 35-55°C
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

	static async getSystemStats(): Promise<SystemStats> {
		if (this.cache && Date.now() < this.cache.expiry) {
			return this.cache.data;
		}

		const data = await this.fetchSystemStats();
		this.cache = {
			data,
			expiry: Date.now() + TIMING_STRATEGIES.FREQUENT.interval
		};
		return data;
	}
}

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
		// Try different temperature sources for Raspberry Pi
		const commands = [
			'cat /sys/class/thermal/thermal_zone0/temp',
			'vcgencmd measure_temp',
			'cat /proc/acpi/thermal_zone/THM0/temperature'
		];

		for (const cmd of commands) {
			try {
				const { stdout } = await execAsync(cmd);
				let temp = stdout.trim();

				// Handle different temperature formats
				if (temp.includes('temp=')) {
					// vcgencmd format: temp=45.2'C
					temp = temp.replace('temp=', '').replace("'C", '');
					return Math.round(parseFloat(temp));
				} else if (temp.length > 3) {
					// thermal_zone format: 45234 (in millidegrees)
					return Math.round(parseInt(temp) / 1000);
				} else {
					// acpi format: 45
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

export const GET: RequestHandler = async () => {
	try {
		const stats = await SystemStatsService.getSystemStats();
		return json(stats);
	} catch (error) {
		console.error('Error fetching system stats:', error);
		return json({ error: 'Failed to fetch system stats' }, { status: 500 });
	}
};
