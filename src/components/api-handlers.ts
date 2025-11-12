// Static imports for all API handlers
// This ensures they're available at runtime in both dev and production builds
// Import first, then use in map to avoid timing issues with module resolution

import { GET as systemStatsApiHandler } from './system-stats/api';
import { GET as remindersApiHandler } from './reminders/api';
import { GET as adguardApiHandler } from './adguard/api';
import { GET as weatherApiHandler } from './weather/api';
import { GET as rssFeedApiHandler } from './rss-feed/api';
import { GET as spotifyApiHandler } from './spotify/api';
import { GET as calendarApiHandler } from './calendar/api';
import { GET as eventsApiHandler } from './events/api';
import { GET as wifiQrCodeApiHandler } from './wifi-qr-code/api';
import { GET as polestarApiHandler } from './polestar/api';

// Map component IDs to their API handlers
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const apiHandlerMap: Record<string, (config: any, request?: Request) => Promise<any>> = {
	'system-stats': systemStatsApiHandler,
	reminders: remindersApiHandler,
	adguard: adguardApiHandler,
	weather: weatherApiHandler,
	'rss-feed': rssFeedApiHandler,
	spotify: spotifyApiHandler,
	calendar: calendarApiHandler,
	events: eventsApiHandler,
	'wifi-qr-code': wifiQrCodeApiHandler,
	polestar: polestarApiHandler
};
