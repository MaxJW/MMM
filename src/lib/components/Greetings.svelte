<script lang="ts">
	import { onMount } from 'svelte';
	import dayjs from 'dayjs';
	import { TIMING_STRATEGIES } from '$lib/types/util';

	let now: dayjs.Dayjs;
	let greeting = '';

	let timer: ReturnType<typeof setInterval> | undefined;

	function updateGreeting() {
		const hour = now.hour();
		if (hour < 12) greeting = 'Sul Sul';
		else if (hour < 18) greeting = 'Sul Sul';
		else greeting = 'Sul Sul';
	}

	onMount(() => {
		// Initialize now only on the client side
		now = dayjs();

		// Set initial values
		updateGreeting();

		// Update every minute for efficiency
		timer = setInterval(() => {
			now = dayjs();
			updateGreeting();
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
