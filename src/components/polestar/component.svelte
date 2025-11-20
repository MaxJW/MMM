<script lang="ts">
	import Car from '@lucide/svelte/icons/car-front';
	import Battery from '@lucide/svelte/icons/battery';
	import Zap from '@lucide/svelte/icons/zap';
	import Gauge from '@lucide/svelte/icons/gauge';
	import AlertTriangle from '@lucide/svelte/icons/triangle-alert';
	import { onMount, onDestroy } from 'svelte';
	import type { PolestarData } from './types';
	import { TIMING_STRATEGIES } from '$lib/core/timing';
	import PolestarNose from './assets/polestar_nose.png';

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

	function getBatteryColor(level: number, status: string): string {
		if (status === 'CHARGING_STATUS_CHARGING' || status === 'CHARGING') return 'text-green-400';
		if (level <= 20) return 'text-yellow-400';
		return 'text-white';
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
		<div class="flex min-h-0 flex-1 items-center gap-4">
			<!-- Left: Car Image (Peaking in) -->
			<div class="relative w-32 flex-shrink-0 overflow-visible">
				<img
					src={PolestarNose}
					alt="Polestar"
					class="h-auto w-full -translate-x-4 scale-125 object-contain"
					style="max-width: none;"
				/>
			</div>

			<!-- Right: Stats Stack -->
			<div class="flex flex-1 flex-col justify-center gap-4 py-2 pr-2">
				<!-- Battery -->
				<div class="flex flex-col gap-0.5">
					<div class="flex items-center gap-2 text-sm opacity-70">
						<Battery size={14} />
						<span>Battery</span>
					</div>
					<div class="flex items-baseline gap-2">
						{#if data.battery.chargingStatus === 'CHARGING_STATUS_CHARGING' || data.battery.chargingStatus === 'CHARGING'}
							<Zap size={16} class="animate-pulse text-yellow-400" />
						{/if}
						<span
							class="text-2xl font-bold {getBatteryColor(
								data.battery.batteryChargeLevelPercentage,
								data.battery.chargingStatus
							)}"
						>
							{data.battery.batteryChargeLevelPercentage}%
						</span>
					</div>
				</div>

				<!-- Range -->
				<div class="flex flex-col gap-0.5">
					<div class="flex items-center gap-2 text-sm opacity-70">
						<Gauge size={14} />
						<span>Range</span>
					</div>
					<div class="text-2xl font-bold">
						{Math.round(data.battery.estimatedDistanceToEmptyKm * 0.621371)}
						<span class="text-base font-normal opacity-70">mi</span>
					</div>
				</div>

				<!-- Odometer -->
				<div class="flex flex-col gap-0.5">
					<div class="flex items-center gap-2 text-sm opacity-70">
						<span>Odometer</span>
					</div>
					<div class="text-lg font-medium opacity-90">
						{Math.round((data.odometer.odometerMeters / 1000) * 0.621371).toLocaleString()}
						<span class="text-sm font-normal opacity-70">mi</span>
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>
