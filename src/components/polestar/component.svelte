<script lang="ts">
	import Car from '@lucide/svelte/icons/car-front';
	import Battery from '@lucide/svelte/icons/battery';
	import Zap from '@lucide/svelte/icons/zap';
	import Gauge from '@lucide/svelte/icons/gauge';
	import AlertTriangle from '@lucide/svelte/icons/triangle-alert';
	import { onMount, onDestroy } from 'svelte';
	import type { PolestarData } from './types';
	import { TIMING_STRATEGIES } from '$lib/core/timing';

	let data: PolestarData | null = null;
	let loading = true;
	let error: string | null = null;
	let refreshInterval: ReturnType<typeof setInterval> | null = null;

	async function load() {
		try {
			const res = await fetch('/api/components/polestar');
			if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);

			const responseData = await res.json();

			if (responseData.error) {
				error = responseData.error;
				data = null;
				return;
			}

			data = responseData;
			error = null;
		} catch (err) {
			error = 'Failed to load Polestar data';
			console.error('Error loading Polestar data:', err);
			data = null;
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		load();
		refreshInterval = setInterval(load, TIMING_STRATEGIES.FREQUENT.interval); // 5 minutes

		return () => {
			if (refreshInterval) clearInterval(refreshInterval);
		};
	});

	onDestroy(() => {
		if (refreshInterval) clearInterval(refreshInterval);
	});

	function getBatteryColor(level: number): string {
		if (level > 50) return 'text-green-400';
		if (level > 20) return 'text-yellow-400';
		return 'text-red-400';
	}
</script>

<div class="flex flex-col gap-4">
	<div class="flex items-center gap-3 opacity-90">
		<Car size={24} />
		<h2 class="text-lg font-medium">Polestar</h2>
	</div>

	{#if loading && !data}
		<div class="flex animate-pulse flex-col gap-2">
			<div class="h-4 w-3/4 rounded bg-white/10"></div>
			<div class="h-4 w-1/2 rounded bg-white/10"></div>
		</div>
	{:else if error}
		<div class="flex items-center gap-2 text-sm text-red-400">
			<AlertTriangle size={16} />
			<span>{error}</span>
		</div>
	{:else if data}
		<div class="flex flex-col gap-3">
			<!-- Battery & Range -->
			<div class="grid grid-cols-2 gap-4">
				<div class="flex flex-col gap-1 rounded-lg bg-white/5 p-3">
					<div class="flex items-center gap-2 text-sm opacity-70">
						<Battery size={16} />
						<span>Battery</span>
					</div>
					<div class="flex items-baseline gap-1">
						<span
							class="text-2xl font-bold {getBatteryColor(
								data.battery.batteryChargeLevelPercentage
							)}"
						>
							{data.battery.batteryChargeLevelPercentage}%
						</span>
						{#if data.battery.chargingStatus === 'CHARGING'}
							<Zap size={16} class="animate-pulse text-yellow-400" />
						{/if}
					</div>
				</div>

				<div class="flex flex-col gap-1 rounded-lg bg-white/5 p-3">
					<div class="flex items-center gap-2 text-sm opacity-70">
						<Gauge size={16} />
						<span>Range</span>
					</div>
					<div class="text-2xl font-bold">
						{Math.round(data.battery.estimatedDistanceToEmptyKm * 0.621371)}
						<span class="text-base font-normal opacity-70">mi</span>
					</div>
				</div>
			</div>

			<!-- Odometer -->
			<div class="flex items-center justify-between px-1 text-sm opacity-70">
				<span>Odometer</span>
				<span
					>{Math.round((data.odometer.odometerMeters / 1000) * 0.621371).toLocaleString()} mi</span
				>
			</div>
		</div>
	{/if}
</div>
