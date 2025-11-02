import type { DashboardConfig, DashboardArea } from '$lib/types/dashboard';
import type { SvelteComponent } from 'svelte';
import Clock from '$lib/components/Clock.svelte';
import Weather from '$lib/components/Weather.svelte';
import Reminders from '$lib/components/Reminders.svelte';
import Calendar from '$lib/components/Calendar.svelte';
import SystemStats from '$lib/components/SystemStats.svelte';
import Greetings from '$lib/components/Greetings.svelte';
import RSSFeed from '$lib/components/RSSFeed.svelte';
import WifiQrCode from '$lib/components/WifiQRCode.svelte';
import AdguardHome from '$lib/components/AdguardHome.svelte';
import EnergyUsage from '$lib/components/EnergyUsage.svelte';
import SpotifyPlayer from '$lib/components/SpotifyPlayer.svelte';
import EventImage from '$lib/components/EventImage.svelte';
import type { DashboardComponentConfig } from './userConfig';

// Map of component IDs to their Svelte component classes
export const componentMap: Record<string, typeof SvelteComponent> = {
	clock: Clock as typeof SvelteComponent,
	weather: Weather as typeof SvelteComponent,
	events: Calendar as typeof SvelteComponent,
	'system-stats': SystemStats as typeof SvelteComponent,
	'wifi-qr-code': WifiQrCode as typeof SvelteComponent,
	'event-image': EventImage as typeof SvelteComponent,
	greetings: Greetings as typeof SvelteComponent,
	reminders: Reminders as typeof SvelteComponent,
	'rss-feed': RSSFeed as typeof SvelteComponent,
	adguard: AdguardHome as typeof SvelteComponent,
	energy: EnergyUsage as typeof SvelteComponent,
	spotify: SpotifyPlayer as typeof SvelteComponent
};

/**
 * Build dashboard config from user config
 * Filters enabled components and maps IDs to components
 * Preserves the order from the JSON array (components appear in the order they're listed)
 */
export function buildDashboardConfig(userConfig: {
	components: DashboardComponentConfig[];
}): DashboardConfig {
	// Filter enabled components and preserve array order
	const enabledComponents = userConfig.components.filter((comp) => comp.enabled);

	const components = enabledComponents
		.map((comp) => {
			const component = componentMap[comp.id];
			if (!component) {
				console.warn(`Component ${comp.id} not found in componentMap`);
				return null;
			}
			return {
				id: comp.id,
				component,
				area: comp.area as DashboardArea
			};
		})
		.filter((comp): comp is NonNullable<typeof comp> => comp !== null);

	return { components };
}
