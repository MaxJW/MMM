<script lang="ts">
	import { onMount } from 'svelte';
	import { TIMING_STRATEGIES } from '$lib/core/timing';
	import { getCurrentEvent } from '$lib/config/events';
	import type { EventConfig } from '$lib/config/events';
	import type { Dayjs } from 'dayjs';

	let currentEvent: (EventConfig & { startDate: Dayjs; endDate: Dayjs }) | null = null;
	let timer: ReturnType<typeof setInterval> | undefined;

	async function updateEvent() {
		currentEvent = await getCurrentEvent();
	}

	onMount(() => {
		// Initialize event on mount
		updateEvent().catch(console.error);

		// Update hourly to check for event changes (events are date-based)
		timer = setInterval(() => {
			updateEvent().catch(console.error);
		}, TIMING_STRATEGIES.INFREQUENT.interval);

		return () => {
			if (timer) clearInterval(timer);
		};
	});
</script>

{#if currentEvent}
	<div class="flex flex-col items-center select-none">
		{#if currentEvent.eventImage}
			<img src={currentEvent.eventImage} alt="Event" />
		{/if}
		<div class="flex flex-col items-center">
			{#if currentEvent.eventText}
				<h2 class="text-xl">{currentEvent.eventText}</h2>
			{/if}
			{#if currentEvent.qrCode}
				<img class="size-60" src={currentEvent.qrCode} alt="QR Code" />
			{/if}
		</div>
	</div>
{/if}
