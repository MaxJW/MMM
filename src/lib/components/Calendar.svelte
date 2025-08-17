<script lang="ts">
	import { CalendarDays, Clock, Circle } from 'lucide-svelte';

	const events: Array<
		{
			day: string;
			date: number;
			month: string;
			title: string;
			time: string;
			isAllDay: boolean;
			category?: 'work' | 'personal';
		}[]
	> = [
		[
			{
				day: 'Fri',
				date: 15,
				month: 'AUG',
				title: "Jack Shield's Birthday",
				time: 'All day',
				isAllDay: true
			},
			{
				day: 'Fri',
				date: 15,
				month: 'AUG',
				title: 'Engineering quick-fire standup',
				time: '9:15 - 9:20',
				isAllDay: false
			},
			{
				day: 'Fri',
				date: 15,
				month: 'AUG',
				title: 'Max <> Jack 1:1',
				time: '9:30 - 10:00',
				isAllDay: false,
				category: 'work'
			},
			{
				day: 'Fri',
				date: 15,
				month: 'AUG',
				title: 'Engineering EOW',
				time: '13:00 - 14:00',
				isAllDay: false,
				category: 'work'
			}
		],
		[
			{
				day: 'Sat',
				date: 16,
				month: 'AUG',
				title: "Danielle's Home Hen",
				time: 'All day',
				isAllDay: true
			},
			{
				day: 'Sat',
				date: 16,
				month: 'AUG',
				title: 'Dinner w/ Ali and Chloe (?)',
				time: 'All day',
				isAllDay: true
			},
			{
				day: 'Sat',
				date: 16,
				month: 'AUG',
				title: 'Jack Matthews Birthday',
				time: 'All day',
				isAllDay: true
			},
			{
				day: 'Sat',
				date: 16,
				month: 'AUG',
				title: "Jack's Birthday",
				time: 'All day',
				isAllDay: true,
				category: 'personal'
			}
		],
		[
			{
				day: 'Sun',
				date: 17,
				month: 'AUG',
				title: 'Matthew Gouldie Birthday',
				time: 'All day',
				isAllDay: true
			},
			{
				day: 'Sun',
				date: 17,
				month: 'AUG',
				title: 'Book Club - Scott',
				time: '12:30 - 18:00',
				isAllDay: false
			}
		]
	];
</script>

<div class="flex flex-col items-start gap-2 select-none">
	<div class="space-y-2">
		{#each events as dayEvents}
			{#if dayEvents.length > 0}
				{@const firstEvent = dayEvents[0]}
				<div class="flex">
					<!-- Date column -->
					<div class="flex min-w-16 flex-col items-center">
						<div class="flex items-center gap-2">
							<!-- <div class="h-2 w-2 rounded-full bg-blue-400"></div> -->
							<div class="text-2xl font-bold">{firstEvent.date}</div>
						</div>
						<div class="text-sm opacity-80">{firstEvent.day}</div>
						<div class="text-sm opacity-80">{firstEvent.month}</div>
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
											<div class="text-base font-medium">{event.title}</div>
											<div class="flex items-center gap-2 text-sm opacity-70">
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
</div>
