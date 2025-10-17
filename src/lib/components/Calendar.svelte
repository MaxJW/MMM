<script lang="ts">
	import { onMount } from 'svelte';
	import Circle from '@lucide/svelte/icons/circle';
	import Clock from '@lucide/svelte/icons/clock';
	import MapPin from '@lucide/svelte/icons/map-pin';
	import {
		CalendarService,
		type CalendarDay,
		type CalendarEvent
	} from '$lib/services/calendarService';
	import { TIMING_STRATEGIES } from '$lib/types/util';
	import CalendarDays from '@lucide/svelte/icons/calendar-days';

	let days: CalendarDay[] | null = null;
	let loading = true;
	let error: string | null = null;
	let firstLoad = true;

	async function loadEvents() {
		try {
			loading = true;
			error = null;
			const result = await CalendarService.getEvents();
			days = result;
		} catch (e) {
			error = 'Failed to load calendar';
		} finally {
			loading = false;
			firstLoad = false;
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

<div class="flex flex-col items-start gap-3 select-none">
	<div class="flex items-center gap-3 opacity-90">
		<CalendarDays size={24} />
		<h2 class="text-lg tracking-wide">Upcoming Events</h2>
	</div>
	{#if firstLoad && loading}
		<p class="text-lg">Loading calendar…</p>
	{:else if error}
		<p class="text-lg text-red-400 opacity-80">{error}</p>
	{:else if !days}
		<button class="rounded bg-green-600 px-3 py-2 text-white" on:click={connectGoogle}>
			Connect Google Calendar
		</button>
	{:else}
		<div class="relative">
			{#each days as dayObj}
				<div class="flex">
					<!-- Date column -->
					<div class="flex min-w-11 flex-col items-center">
						<div class="text-3xl font-bold">{dayObj.date}</div>
						<div class="text-base font-medium opacity-90">{dayObj.day}</div>
					</div>
					<!-- Events column -->
					<div class="flex-1 pb-3 pl-4">
						{#if !dayObj.events.length}
							<div class="relative">
								<!-- Grey timeline bar for no events -->
								<div
									class="absolute top-0 bottom-0 left-0 w-0.5 rounded-t rounded-b bg-gray-400 opacity-70"
								></div>
								<div class="flex min-h-[52px] items-center pl-3 opacity-60">
									<div class="text-lg">No events</div>
								</div>
							</div>
						{:else}
							{#each dayObj.events as event, i (event)}
								<div class="relative">
									<!-- Event-specific timeline bar -->
									<div
										class="absolute top-0 bottom-0 left-0 w-0.5"
										class:bg-green-500={event.category === 'personal'}
										class:bg-blue-500={event.category === 'work'}
										class:bg-gray-400={!event.category}
										class:rounded-t={i === 0}
										class:rounded-b={i === dayObj.events.length - 1}
									></div>
									<div class="flex items-start pl-3">
										<div class="flex-1">
											<div class="text-lg">{event.title}</div>
											<div class="flex items-center gap-2 text-base opacity-70">
												<div class="flex-shrink-0">
													{#if event.isAllDay}
														<Circle size={16} class="stroke-3 opacity-70" />
													{:else}
														<Clock size={16} class="stroke-3 opacity-70" />
													{/if}
												</div>
												{event.time}
											</div>
											{#if event.location}
												<div class="flex items-center gap-2 text-base opacity-70">
													<div class="flex-shrink-0">
														<MapPin size={16} class="stroke-3 opacity-70" />
													</div>
													{event.location}
												</div>
											{/if}
										</div>
									</div>
								</div>
							{/each}
						{/if}
					</div>
				</div>
			{/each}
			<div
				class="pointer-events-none absolute right-0 bottom-0 left-0 h-2/6 bg-gradient-to-t from-black to-transparent"
			></div>
		</div>
	{/if}
</div>
