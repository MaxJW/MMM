import type { DashboardConfig } from '$lib/types/dashboard';
import Clock from '$lib/components/Clock.svelte';
import Weather from '$lib/components/Weather.svelte';
import Reminders from '$lib/components/Reminders.svelte';
import Events from '$lib/components/Events.svelte';
import SystemStats from '$lib/components/SystemStats.svelte';
import Greetings from '$lib/components/Greetings.svelte';
import RSSFeed from '$lib/components/RSSFeed.svelte';

export const dashboardConfig: DashboardConfig = {
	components: [
		{
			id: 'clock',
			component: Clock,
			area: 'top-left'
		},
		{
			id: 'weather',
			component: Weather,
			area: 'top-right'
		},
		{
			id: 'events',
			component: Events,
			area: 'top-left'
		},
		{
			id: 'system-stats',
			component: SystemStats,
			area: 'bottom-right'
		},
		{
			id: 'greetings',
			component: Greetings,
			area: 'center'
		},
		{
			id: 'reminders',
			component: Reminders,
			area: 'notifications'
		},
		{
			id: 'rss-feed',
			component: RSSFeed,
			area: 'notifications'
		}
	]
};
