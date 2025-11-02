<script lang="ts">
	import { onMount } from 'svelte';
	import dayjs from 'dayjs';
	import { TIMING_STRATEGIES } from '$lib/types/util';
	import { getCurrentEventGreeting } from '$lib/config/events';

	let now: dayjs.Dayjs;
	let greeting = '';
	let customGreeting: string | null = null;

	let timer: ReturnType<typeof setInterval> | undefined;

	async function updateGreeting() {
		// Check for custom greeting from active event first
		customGreeting = await getCurrentEventGreeting();

		if (customGreeting) {
			greeting = customGreeting;
			return;
		}

		// Fall back to time-based greeting
		const hour = now.hour();
		if (hour < 12) greeting = 'Good morning';
		else if (hour < 18) greeting = 'Good afternoon';
		else greeting = 'Good evening';
	}

	onMount(() => {
		// Initialize now only on the client side
		now = dayjs();

		// Set initial values
		updateGreeting().catch(console.error);

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
		{greeting || 'Loading...'}!
	</div>
</div>
