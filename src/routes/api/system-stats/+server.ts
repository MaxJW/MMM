import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { exec } from 'child_process';
import { promisify } from 'util';
import { dev } from '$app/environment';

const execAsync = promisify(exec);

interface SystemStats {
	cpu: number;
	memory: number;
	disk: number;
	tempC: number;
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
		// Return placeholder data in development mode
		if (dev) {
			const mockStats: SystemStats = {
				cpu: 23,
				memory: 45,
				disk: 67,
				tempC: 42
			};
			return json(mockStats);
		}

		const [cpu, memory, disk, tempC] = await Promise.all([
			getCpuUsage(),
			getMemoryUsage(),
			getDiskUsage(),
			getTemperature()
		]);

		const stats: SystemStats = {
			cpu,
			memory,
			disk,
			tempC
		};

		return json(stats);
	} catch (error) {
		console.error('Error fetching system stats:', error);
		return json({ error: 'Failed to fetch system stats' }, { status: 500 });
	}
};
