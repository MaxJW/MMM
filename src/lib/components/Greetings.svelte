<script lang="ts">
	import { onMount } from 'svelte';
	import dayjs from 'dayjs';

	let now = dayjs();
	let greeting = '';

	let timer: ReturnType<typeof setInterval> | undefined;

	function updateGreeting() {
		const hour = now.hour();
		if (hour < 12) greeting = 'Good morning';
		else if (hour < 18) greeting = 'Good afternoon';
		else greeting = 'Good evening';
	}

	onMount(() => {
		// Set initial values
		updateGreeting();

		// Update every minute instead of every second for efficiency
		timer = setInterval(() => {
			now = dayjs();
			updateGreeting();
		}, 60_000);

		return () => {
			if (timer) clearInterval(timer);
		};
	});
</script>

<div class="flex flex-col items-center gap-4 select-none">
	<div class="text-center text-4xl font-semibold tabular-nums opacity-80">
		{greeting}!
	</div>
</div>
