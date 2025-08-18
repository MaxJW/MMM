<script lang="ts">
	import { onMount } from 'svelte';
	import { CalendarDays, Clock, Circle } from 'lucide-svelte';
	import { CalendarService, type CalendarEvent } from '$lib/services/calendarService';
	import { TIMING_STRATEGIES } from '$lib/types/util';

	let events: CalendarEvent[][] | null = null;
	let loading = true;
	let error: string | null = null;

	async function loadEvents() {
		try {
			loading = true;
			error = null;
			events = await CalendarService.getEvents();
		} catch (e) {
			error = 'Failed to load calendar';
		} finally {
			loading = false;
		}
	}

	function connectGoogle() {
		window.location.href = '/api/google/auth';
	}

	onMount(() => {
		loadEvents();
		const interval = setInterval(loadEvents, TIMING_STRATEGIES.FREQUENT.interval);
		return () => clearInterval(interval);
	});
</script>

<div class="flex flex-col items-start gap-2 select-none">
	{#if loading}
		<p class="text-lg">Loading calendar…</p>
	{:else if error}
		<p class="text-lg text-red-400 opacity-80">{error}</p>
	{:else if !events}
		<button class="rounded bg-green-600 px-3 py-2 text-white" on:click={connectGoogle}>
			Connect Google Calendar
		</button>
	{:else}
		<div class="space-y-2">
			{#each events as dayEvents}
				{#if dayEvents.length > 0}
					{@const firstEvent = dayEvents[0]}
					<div class="flex">
						<!-- Date column -->
						<div class="flex min-w-16 flex-col items-center">
							<div class="flex items-center gap-2">
								<!-- <div class="h-2 w-2 rounded-full bg-blue-400"></div> -->
								<div class="text-3xl font-bold">{firstEvent.date}</div>
							</div>
							<div class="text-base opacity-90">{firstEvent.day}</div>
							<div class="text-base opacity-90">{firstEvent.month}</div>
						</div>

						<!-- Events column with timeline -->
						<div class="relative flex-1">
							<!-- Main green timeline -->
							<div class="absolute top-0 bottom-0 left-0 w-0.5 bg-green-500"></div>

							<div class="space-y-2 pl-4">
								{#each dayEvents as event, index}
									<div class="relative">
										<div class="flex items-start gap-3">
											<div class="flex-1">
												<div class="text-lg font-medium">{event.title}</div>
												<div class="flex items-center gap-2 text-base opacity-70">
													<div class="flex-shrink-0">
														{#if event.isAllDay}
															<Circle size={16} class="opacity-70" />
														{:else}
															<Clock size={16} class="opacity-70" />
														{/if}
													</div>
													{event.time}
												</div>
											</div>
										</div>
									</div>
								{/each}
							</div>
						</div>
					</div>
				{/if}
			{/each}
		</div>
	{/if}
</div>
