<script lang="ts">
	import { onMount } from 'svelte';
	import CircleAlert from '@lucide/svelte/icons/circle-alert';
	import { TIMING_STRATEGIES } from '$lib/core/timing';
	import { getCurrentEvent } from '$lib/config/events';
	import type { EventConfig } from '$lib/config/events';
	import type { Dayjs } from 'dayjs';

	let currentEvent: (EventConfig & { startDate: Dayjs; endDate: Dayjs }) | null = null;
	let error: string | null = null;
	let timer: ReturnType<typeof setInterval> | undefined;

	async function updateEvent() {
		try {
			error = null;
			currentEvent = await getCurrentEvent();
		} catch (err) {
			error = 'Failed to load event';
			console.error('Error loading event:', err);
			currentEvent = null;
		}
	}

	onMount(() => {
		// Initialize event on mount
		updateEvent();

		// Update hourly to check for event changes (events are date-based)
		timer = setInterval(() => {
			updateEvent();
		}, TIMING_STRATEGIES.INFREQUENT.interval);

		return () => {
			if (timer) clearInterval(timer);
		};
	});
</script>

{#if error}
	<div class="flex items-center gap-2 text-lg text-red-400 opacity-80">
		<CircleAlert size={22} />
		<span>{error}</span>
	</div>
{:else if currentEvent}
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
