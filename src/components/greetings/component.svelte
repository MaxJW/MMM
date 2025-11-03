<script lang="ts">
	import { onMount } from 'svelte';
	import dayjs from 'dayjs';
	import { TIMING_STRATEGIES } from '$lib/core/timing';
	import { getCurrentEventGreeting } from '$lib/config/events';

	interface GreetingConfig {
		text: string;
		startHour: number;
		endHour: number;
	}

	interface GreetingsComponentConfig {
		greetings?: GreetingConfig[];
	}

	let now: dayjs.Dayjs;
	let greeting = '';
	let customGreeting: string | null = null;
	let greetingsConfig: GreetingsComponentConfig = {};

	let timer: ReturnType<typeof setInterval> | undefined;

	// Default greetings (used if config is not available or empty)
	const defaultGreetings: GreetingConfig[] = [
		{ text: 'Good morning', startHour: 0, endHour: 12 },
		{ text: 'Good afternoon', startHour: 12, endHour: 18 },
		{ text: 'Good evening', startHour: 18, endHour: 24 }
	];

	async function loadConfig() {
		try {
			const res = await fetch('/api/config');
			if (res.ok) {
				const config = await res.json();
				greetingsConfig = (config.components?.greetings as GreetingsComponentConfig) ?? {};
			}
		} catch (error) {
			console.error('Failed to load greetings config:', error);
			// Will fall back to defaultGreetings if config fails to load
		}
	}

	function getTimeBasedGreeting(hour: number): string {
		// Get greetings from config or use defaults
		const greetings =
			greetingsConfig.greetings && greetingsConfig.greetings.length > 0
				? greetingsConfig.greetings
				: defaultGreetings;

		// Find the first greeting that matches the current hour
		for (const greetingConfig of greetings) {
			const start = greetingConfig.startHour;
			const end = greetingConfig.endHour;

			// Handle time ranges that span midnight (e.g., 22-6)
			if (start <= end) {
				// Normal range (e.g., 0-12, 12-18)
				if (hour >= start && hour < end) {
					return greetingConfig.text;
				}
			} else {
				// Wraps around midnight (e.g., 22-6)
				if (hour >= start || hour < end) {
					return greetingConfig.text;
				}
			}
		}

		// Fallback (shouldn't happen if config is valid)
		return defaultGreetings[0].text;
	}

	async function updateGreeting() {
		// Check for custom greeting from active event first
		customGreeting = await getCurrentEventGreeting();

		if (customGreeting) {
			greeting = customGreeting;
			return;
		}

		// Fall back to time-based greeting from config
		const hour = now.hour();
		greeting = getTimeBasedGreeting(hour);
	}

	onMount(() => {
		// Initialize now only on the client side
		now = dayjs();

		// Load config first, then set initial greeting
		loadConfig()
			.then(() => updateGreeting())
			.catch(console.error);

		// Update every minute for efficiency
		timer = setInterval(() => {
			now = dayjs();
			updateGreeting().catch(console.error);
		}, TIMING_STRATEGIES.UI.MINUTE);

		return () => {
			if (timer) clearInterval(timer);
		};
	});
</script>

<div class="flex flex-col items-center gap-4 select-none">
	<div class="text-center text-5xl font-semibold tabular-nums opacity-90">
		{greeting || 'Loading...'}
	</div>
</div>
