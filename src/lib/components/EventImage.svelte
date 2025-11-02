<script lang="ts">
	import { onMount } from 'svelte';
	import { TIMING_STRATEGIES } from '$lib/types/util';
	import { getCurrentEvent } from '$lib/config/events';

	let currentEvent: ReturnType<typeof getCurrentEvent> = null;
	let timer: ReturnType<typeof setInterval> | undefined;

	function updateEvent() {
		currentEvent = getCurrentEvent();
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
