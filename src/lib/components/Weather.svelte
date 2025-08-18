<script lang="ts">
	import { onMount } from 'svelte';
	import { CircleAlert, type Icon } from 'lucide-svelte';
	import {
		CloudFog,
		CloudHail,
		CloudLightning,
		CloudMoon,
		CloudRain,
		CloudSun,
		CloudSunRain,
		Cloudy,
		Snowflake,
		Sun,
		CloudOff,
		Moon,
		Wind
	} from 'lucide-svelte';
	import { WeatherService } from '$lib/services/weatherService';
	import type { MeteoAlert, WeatherData, WeatherIcon } from '$lib/types/weather';
	import { TIMING_STRATEGIES } from '$lib/types/util';

	let weather: WeatherData | null = null;
	let alerts: MeteoAlert[] | null = null;
	let weatherLoading = true;
	let alertsLoading = true;
	let weatherError: string | null = null;
	let alertsError: string | null = null;

	const iconMap: Record<WeatherIcon, typeof Icon> = {
		'clear-day': Sun,
		'clear-night': Moon,
		rain: CloudRain,
		snow: Snowflake,
		sleet: CloudHail,
		wind: Wind,
		fog: CloudFog,
		cloudy: Cloudy,
		'partly-cloudy-day': CloudSun,
		'partly-cloudy-night': CloudMoon,
		thunderstorm: CloudLightning,
		hail: CloudHail,
		mixed: CloudSunRain,
		none: CloudOff // Not available
	};

	async function loadWeather() {
		try {
			weatherLoading = true;
			weatherError = null;
			weather = await WeatherService.getWeather();
		} catch (err) {
			weatherError = 'Failed to load weather';
			console.error('Error loading weather:', err);
		} finally {
			weatherLoading = false;
		}
	}

	async function loadAlerts() {
		try {
			alertsLoading = true;
			alertsError = null;
			alerts = await WeatherService.getAlerts();
		} catch (err) {
			alertsError = 'Failed to load alerts';
			console.error('Error loading alerts:', err);
		} finally {
			alertsLoading = false;
		}
	}

	// Map severity to tailwind bg/text colors
	function getAlertColor(severity: string) {
		switch (severity.toLowerCase()) {
			case 'green':
				return 'bg-green-200 text-black';
			case 'yellow':
				return 'bg-yellow-400 text-black';
			case 'amber':
				return 'bg-orange-500 text-white';
			case 'red':
				return 'bg-red-600 text-white';
			default:
				return 'bg-gray-300 text-black';
		}
	}

	function formatDate(dateStr: string) {
		const date = new Date(dateStr);
		return date.toLocaleString(undefined, {
			day: 'numeric',
			month: 'short',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	onMount(() => {
		loadWeather();
		loadAlerts();
		const interval = setInterval(() => {
			loadWeather();
			loadAlerts();
		}, TIMING_STRATEGIES.STANDARD.interval);
		return () => clearInterval(interval);
	});
</script>

{#if weatherLoading}
	<p class="text-xl">Loading weather…</p>
{:else if weatherError}
	<p class="text-xl text-red-400 opacity-90">{weatherError}</p>
{:else if weather}
	<div class="flex flex-col items-end select-none">
		<div class="flex items-center gap-6">
			<div>
				<svelte:component this={iconMap[weather.current.icon]} size={80} />
			</div>
			<div class="text-8xl font-semibold tabular-nums">{weather.current.tempC}°C</div>
		</div>
		<div class="my-3 max-w-[24rem] text-right text-2xl leading-snug opacity-90">
			{weather.current.condition}
		</div>
		<div class="flex items-center gap-10">
			{#each weather.forecast as day}
				<div class="flex flex-col items-center gap-3">
					<div class="text-xl opacity-80">{day.day}</div>
					<svelte:component this={iconMap[day.icon]} size={36} />
					<div class="text-xl tabular-nums">
						<span>{day.hi}°</span>
						<span class="opacity-70">/ {day.lo}°</span>
					</div>
				</div>
			{/each}
		</div>
	</div>
{/if}

{#if alertsLoading}
	<!-- Alerts loading state handled by weather loading -->
{:else if alertsError}
	<p class="text-base text-red-400 opacity-90">{alertsError}</p>
{:else if alerts && alerts.length > 0}
	<div class="flex flex-col items-end gap-4 select-none">
		{#each alerts as alert}
			<div
				class={`rounded-lg p-4 shadow-md ${getAlertColor(alert.severity)}`}
				style="width: 350px;"
			>
				<!-- Title with exclamation icon -->
				<div class="mb-2 flex items-center gap-4">
					<CircleAlert size={30} class="flex-shrink-0" />
					<div class="flex-1 text-xl leading-tight font-semibold">
						{alert.event}
						<div class="text-sm opacity-80">
							{formatDate(alert.start)} – {formatDate(alert.end)}
						</div>
					</div>
				</div>

				<!-- Full description -->
				<div class="mb-2 text-base leading-snug break-words">
					{alert.description ?? alert.instruction}
				</div>
			</div>
		{/each}
	</div>
{/if}
