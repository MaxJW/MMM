<script lang="ts">
	import { onMount } from 'svelte';
	import { TIMING_STRATEGIES } from '$lib/core/timing';
	import type { PolestarVehicleData } from './types';
	import Battery from '@lucide/svelte/icons/battery';
	import Car from '@lucide/svelte/icons/car';
	import Gauge from '@lucide/svelte/icons/gauge';
	import AlertCircle from '@lucide/svelte/icons/alert-circle';
	import CheckCircle from '@lucide/svelte/icons/check-circle';
	import Zap from '@lucide/svelte/icons/zap';
	import MapPin from '@lucide/svelte/icons/map-pin';
	import Wrench from '@lucide/svelte/icons/wrench';

	let vehicleData: PolestarVehicleData | null = null;
	let loading = true;
	let error: string | null = null;
	let timer: ReturnType<typeof setInterval> | undefined;

	async function loadVehicleData() {
		try {
			loading = true;
			error = null;
			const res = await fetch('/api/components/polestar');
			if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);

			const data = await res.json();

			if (data && typeof data === 'object' && 'error' in data) {
				error = data.error;
				vehicleData = null;
			} else {
				vehicleData = data;
			}
		} catch (err) {
			error = 'Failed to load vehicle data';
			vehicleData = null;
			console.error('Error loading Polestar data:', err);
		} finally {
			loading = false;
		}
	}

	function formatDistance(km?: number): string {
		if (km === undefined || km === null) return 'N/A';
		return `${Math.round(km)} km`;
	}

	function formatPercentage(percent?: number): string {
		if (percent === undefined || percent === null) return 'N/A';
		return `${Math.round(percent)}%`;
	}

	function formatTime(minutes?: number): string {
		if (minutes === undefined || minutes === null) return 'N/A';
		const hours = Math.floor(minutes / 60);
		const mins = minutes % 60;
		if (hours > 0) {
			return `${hours}h ${mins}m`;
		}
		return `${mins}m`;
	}

	function formatConsumption(kwhPer100km?: number): string {
		if (kwhPer100km === undefined || kwhPer100km === null) return 'N/A';
		return `${kwhPer100km.toFixed(1)} kWh/100km`;
	}

	onMount(() => {
		loadVehicleData();
		timer = setInterval(() => {
			loadVehicleData();
		}, TIMING_STRATEGIES.MEDIUM.interval);
		return () => {
			if (timer) clearInterval(timer);
		};
	});
</script>

<div class="flex flex-col items-center gap-4 select-none">
	{#if loading}
		<div class="flex items-center gap-2 text-2xl opacity-80">
			<Car size={22} />
			<span>Loading vehicle data...</span>
		</div>
	{:else if error}
		<div class="flex items-center gap-2 text-2xl opacity-80">
			<AlertCircle size={22} />
			<span>{error}</span>
		</div>
	{:else if vehicleData}
		<!-- Vehicle Model -->
		{#if vehicleData.carInformation?.modelName}
			<div class="flex items-center gap-2 text-2xl font-medium">
				<Car size={28} class="opacity-80" />
				<span>{vehicleData.carInformation.modelName}</span>
			</div>
		{/if}

		<!-- Battery Information -->
		{#if vehicleData.carBattery}
			<div class="flex flex-col items-center gap-2">
				<div class="flex items-center gap-2 text-xl">
					<Battery size={24} class="opacity-80" />
					<span class="font-medium">
						{formatPercentage(vehicleData.carBattery.batteryChargeLevelPercentage)}
					</span>
				</div>
				{#if vehicleData.carBattery.estimatedDistanceToEmptyKm !== undefined}
					<div class="flex items-center gap-2 text-lg opacity-90">
						<MapPin size={20} class="opacity-70" />
						<span>{formatDistance(vehicleData.carBattery.estimatedDistanceToEmptyKm)}</span>
					</div>
				{/if}
				{#if vehicleData.carBattery.chargingStatus === 'CHARGING'}
					<div class="flex items-center gap-2 text-lg opacity-90">
						<Zap size={20} class="opacity-70" />
						<span>Charging</span>
						{#if vehicleData.carBattery.estimatedChargingTimeToFullMinutes !== undefined}
							<span class="opacity-70">
								({formatTime(vehicleData.carBattery.estimatedChargingTimeToFullMinutes)})
							</span>
						{/if}
					</div>
				{/if}
				{#if vehicleData.carBattery.averageEnergyConsumptionKwhPer100km !== undefined}
					<div class="flex items-center gap-2 text-sm opacity-80">
						<Gauge size={16} class="opacity-70" />
						<span
							>{formatConsumption(vehicleData.carBattery.averageEnergyConsumptionKwhPer100km)}</span
						>
					</div>
				{/if}
			</div>
		{/if}

		<!-- Odometer -->
		{#if vehicleData.carOdometer?.odometerMeters !== undefined}
			<div class="flex items-center gap-2 text-lg opacity-90">
				<Gauge size={20} class="opacity-70" />
				<span>
					{formatDistance((vehicleData.carOdometer.odometerMeters || 0) / 1000)}
				</span>
			</div>
		{/if}

		<!-- Health/Service Warnings -->
		{#if vehicleData.carHealth}
			{#if vehicleData.carHealth.serviceWarning || vehicleData.carHealth.daysToService !== undefined}
				<div class="flex items-center gap-2 text-lg opacity-90">
					<Wrench size={20} class="opacity-70" />
					{#if vehicleData.carHealth.serviceWarning}
						<span class="text-yellow-400">Service Required</span>
					{:else if vehicleData.carHealth.daysToService !== undefined}
						<span>
							Service in {vehicleData.carHealth.daysToService} day{vehicleData.carHealth
								.daysToService !== 1
								? 's'
								: ''}
						</span>
					{/if}
				</div>
			{/if}
		{/if}

		<!-- API Status (debug info, can be hidden) -->
		{#if vehicleData.apiConnected === false}
			<div class="flex items-center gap-2 text-sm opacity-60">
				<AlertCircle size={16} />
				<span>API disconnected</span>
			</div>
		{:else if vehicleData.apiConnected === true}
			<div class="flex items-center gap-2 text-sm opacity-60">
				<CheckCircle size={16} />
				<span>Connected</span>
			</div>
		{/if}
	{/if}
</div>
