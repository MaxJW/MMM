<script lang="ts">
	import { onMount } from 'svelte';
	import Circle from '@lucide/svelte/icons/circle';
	import Clock from '@lucide/svelte/icons/clock';
	import MapPin from '@lucide/svelte/icons/map-pin';
	import CircleAlert from '@lucide/svelte/icons/circle-alert';
	import type { CalendarDay, CalendarEvent } from './types';
	import { TIMING_STRATEGIES } from '$lib/core/timing';
	import CalendarDays from '@lucide/svelte/icons/calendar-days';
	import { dev } from '$app/environment';

	// Color mapping for Tailwind classes
	// Tailwind requires full class names to be present in the source code
	const colorClassMap: Record<string, string> = {
		'gray-400': 'bg-gray-400',
		'gray-500': 'bg-gray-500',
		'green-500': 'bg-green-500',
		'green-600': 'bg-green-600',
		'blue-500': 'bg-blue-500',
		'blue-600': 'bg-blue-600',
		'purple-500': 'bg-purple-500',
		'purple-600': 'bg-purple-600',
		'red-500': 'bg-red-500',
		'red-600': 'bg-red-600',
		'yellow-500': 'bg-yellow-500',
		'yellow-600': 'bg-yellow-600',
		'orange-500': 'bg-orange-500',
		'orange-600': 'bg-orange-600',
		'pink-500': 'bg-pink-500',
		'pink-600': 'bg-pink-600',
		'indigo-500': 'bg-indigo-500',
		'indigo-600': 'bg-indigo-600',
		'teal-500': 'bg-teal-500',
		'teal-600': 'bg-teal-600',
		'cyan-500': 'bg-cyan-500',
		'cyan-600': 'bg-cyan-600'
	};

	function getBgClass(colorClass?: string): string {
		if (!colorClass) return colorClassMap['gray-400'];
		return colorClassMap[colorClass] || colorClassMap['gray-400'];
	}

	let days: CalendarDay[] | null = null;
	let loading = true;
	let error: string | null = null;
	let firstLoad = true;

	async function loadEvents() {
		try {
			loading = true;
			error = null;

			if (dev) {
				// Return empty calendar in dev mode - use real Google Calendar for testing
				days = [];
				return;
			}

			const res = await fetch('/api/google/events');
			if (res.status === 401) {
				days = null;
				return;
			}
			if (!res.ok) throw new Error(`HTTP ${res.status}`);

			const result = await res.json();
			days = result;
		} catch (e) {
			error = 'Failed to load calendar';
			console.error('Failed to fetch calendar events:', e);
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
		<div class="flex items-center gap-2 text-lg text-red-400 opacity-80">
			<CircleAlert size={22} />
			<span>{error}</span>
		</div>
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
										class="absolute top-0 bottom-0 left-0 w-0.5 {getBgClass(event.colorClass)}"
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
