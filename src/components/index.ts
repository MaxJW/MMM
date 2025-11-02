// Explicit component imports - more reliable than glob patterns
import ClockComponent from './clock/component.svelte';
import WeatherComponent from './weather/component.svelte';
import CalendarComponent from './calendar/component.svelte';
import SystemStatsComponent from './system-stats/component.svelte';
import WifiQrCodeComponent from './wifi-qr-code/component.svelte';
import EventsComponent from './events/component.svelte';
import GreetingsComponent from './greetings/component.svelte';
import RemindersComponent from './reminders/component.svelte';
import RssFeedComponent from './rss-feed/component.svelte';
import AdguardComponent from './adguard/component.svelte';
import EnergyComponent from './energy/component.svelte';
import SpotifyComponent from './spotify/component.svelte';

/**
 * Component registry - maps component IDs to their Svelte components
 * This is more reliable than glob patterns and better for build tools
 */
export const componentMap: Record<string, unknown> = {
	clock: ClockComponent,
	weather: WeatherComponent,
	calendar: CalendarComponent,
	'system-stats': SystemStatsComponent,
	'wifi-qr-code': WifiQrCodeComponent,
	events: EventsComponent,
	greetings: GreetingsComponent,
	reminders: RemindersComponent,
	'rss-feed': RssFeedComponent,
	adguard: AdguardComponent,
	energy: EnergyComponent,
	spotify: SpotifyComponent
};
