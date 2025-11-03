import { google, type calendar_v3 } from 'googleapis';
import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';
dayjs.extend(utc);
import { TIMING_STRATEGIES } from '$lib/core/timing';
import { TokenStorage } from '$lib/services/tokenStorage';

type SimplifiedEvent = {
	day: string;
	date: number;
	month: string;
	title: string;
	time: string;
	location?: string;
	isAllDay: boolean;
	calendarName?: string;
	colorClass?: string;
};

type CalendarListEntryLite = {
	id?: string;
	deleted?: boolean;
	selected?: boolean;
	primary?: boolean;
	summary?: string;
};

type GoogleEventLite = {
	summary?: string;
	start?: { dateTime?: string; date?: string };
	end?: { dateTime?: string; date?: string };
	location?: string;
};

interface CalendarConfig {
	clientId?: string;
	clientSecret?: string;
	maxEvents?: number;
	calendarColors?: Array<{ calendarName: string; colorClass: string }> | Record<string, string>;
}

class GoogleCalendarService {
	private static cache: {
		data: Array<{ day: string; date: number; month: string; events: SimplifiedEvent[] }>;
		expiry: number;
		maxEvents: number;
		calendarColorsHash: string;
	} | null = null;

	static async getEvents(
		origin: string,
		config: CalendarConfig
	): Promise<Array<{ day: string; date: number; month: string; events: SimplifiedEvent[] }>> {
		// Load tokens from file
		const tokenData = await TokenStorage.loadTokens();
		if (!tokenData) {
			throw new Error('Not authenticated');
		}

		if (!config.clientId || !config.clientSecret) {
			throw new Error('Google OAuth not configured');
		}

		// Get maxEvents from config (default to 12)
		const maxEvents = typeof config.maxEvents === 'number' ? config.maxEvents : 12;

		// Get calendarColors from config (default to empty array)
		// Support both new array format and legacy object format for backward compatibility
		let calendarColorsArray: Array<{ calendarName: string; colorClass: string }> = [];
		if (Array.isArray(config.calendarColors)) {
			calendarColorsArray = config.calendarColors;
		} else if (config.calendarColors && typeof config.calendarColors === 'object') {
			// Legacy object format: convert to array format
			calendarColorsArray = Object.entries(config.calendarColors).map(
				([calendarName, colorClass]) => ({
					calendarName,
					colorClass: String(colorClass)
				})
			);
		}

		// Create a hash of calendarColors for cache invalidation
		const calendarColorsHash = JSON.stringify(calendarColorsArray);

		// Helper function to get color class for a calendar name
		const getColorClass = (calendarName?: string): string => {
			if (!calendarName) return 'gray-400';

			// Check for exact match first
			const exactMatch = calendarColorsArray.find((entry) => entry.calendarName === calendarName);
			if (exactMatch) {
				return exactMatch.colorClass;
			}

			// Check for case-insensitive match
			const lowerName = calendarName.toLowerCase();
			const caseInsensitiveMatch = calendarColorsArray.find(
				(entry) => entry.calendarName.toLowerCase() === lowerName
			);
			if (caseInsensitiveMatch) {
				return caseInsensitiveMatch.colorClass;
			}

			// Default fallback colors based on common calendar names
			if (lowerName.includes('work') || lowerName.includes('business')) {
				return 'blue-500';
			}
			if (lowerName.includes('personal') || lowerName.includes('private')) {
				return 'green-500';
			}
			if (lowerName.includes('family')) {
				return 'purple-500';
			}
			return 'gray-400';
		};

		// Check cache - invalidate if maxEvents or calendarColors changed
		if (
			this.cache &&
			Date.now() < this.cache.expiry &&
			this.cache.maxEvents === maxEvents &&
			this.cache.calendarColorsHash === calendarColorsHash
		) {
			return this.cache.data;
		}

		const oauth2Client = new google.auth.OAuth2({
			clientId: config.clientId,
			clientSecret: config.clientSecret,
			redirectUri: `${origin}/api/google/callback`
		});

		// Set credentials
		oauth2Client.setCredentials({
			refresh_token: tokenData.refreshToken,
			access_token: tokenData.accessToken,
			expiry_date: tokenData.expiryDate
		});

		try {
			// This will automatically refresh the access token if needed
			const { token } = await oauth2Client.getAccessToken();

			// Save updated tokens if they changed
			if (token !== tokenData.accessToken) {
				await TokenStorage.saveTokens({
					...tokenData,
					accessToken: token || undefined,
					expiryDate: oauth2Client.credentials.expiry_date || undefined
				});
			}
		} catch (err) {
			console.error('Failed to refresh Google access token', err);
			throw new Error('Authentication failed');
		}

		const calendar: calendar_v3.Calendar = google.calendar({ version: 'v3', auth: oauth2Client });

		const calList = await calendar.calendarList.list();
		const itemsUnknown = (calList.data.items || []) as unknown[];
		const calendars: CalendarListEntryLite[] = itemsUnknown
			.map((c) => c as CalendarListEntryLite)
			.filter((c) => !c.deleted && c.selected !== false && !!c.id);

		// Log calendar names to help with configuration
		const calendarNames = calendars
			.map((cal) => cal.summary || cal.id || 'Unknown')
			.filter((name) => name !== 'Unknown');
		console.log(
			'[Calendar] Available calendars for color mapping:',
			JSON.stringify(calendarNames, null, 2)
		);

		const now = dayjs();
		const end = now.add(3, 'day');

		const allEvents = await Promise.all(
			calendars.map(async (cal) => {
				const res = await calendar.events.list({
					calendarId: cal.id as string,
					timeMin: now.toISOString(),
					timeMax: end.toISOString(),
					singleEvents: true,
					orderBy: 'startTime',
					eventTypes: ['birthday', 'default', 'focusTime', 'fromGmail', 'outOfOffice']
				});
				const eventsUnknown = (res.data.items || []) as unknown[];
				return eventsUnknown.map((e) => ({ event: e as GoogleEventLite, calendar: cal }));
			})
		);

		// Flatten the array of arrays into a single array of all events
		const flattenedEvents = allEvents.flat() as Array<{
			event: GoogleEventLite;
			calendar: CalendarListEntryLite;
		}>;

		// Sort all events by their start time
		flattenedEvents.sort((a, b) => {
			const aStart = dayjs(a.event.start?.dateTime || a.event.start?.date);
			const bStart = dayjs(b.event.start?.dateTime || b.event.start?.date);
			return aStart.diff(bStart);
		});

		// Convert all events to simplified format first, grouped by day
		const allGroupedEvents: Record<string, SimplifiedEvent[]> = {};
		for (const { event, calendar: cal } of flattenedEvents) {
			const start = event.start?.dateTime || event.start?.date;
			if (!start) continue;
			const d = dayjs(start);
			const isAllDay = !event.start?.dateTime;
			const startTime = event.start?.dateTime ? dayjs(event.start.dateTime) : null;
			const endTime = event.end?.dateTime ? dayjs(event.end.dateTime) : null;
			const time =
				isAllDay || !startTime || !endTime
					? 'All day'
					: `${startTime.format('H:mm')} - ${endTime.format('H:mm')}`;
			const key = d.format('YYYY-MM-DD');
			const month = d.format('MMM').toUpperCase();
			const calendarName = cal.summary || undefined;
			const simplified: SimplifiedEvent = {
				day: d.format('ddd'),
				date: parseInt(d.format('D')),
				month,
				title: event.summary || '(No title)',
				time,
				location: event.location ? event.location.split(',')[0] : undefined,
				isAllDay,
				calendarName,
				colorClass: getColorClass(calendarName)
			};
			if (!allGroupedEvents[key]) allGroupedEvents[key] = [];
			allGroupedEvents[key].push(simplified);
		}

		// Process days sequentially, counting "No events" days as 1 toward the limit
		const result: Array<{ day: string; date: number; month: string; events: SimplifiedEvent[] }> =
			[];
		let itemsCount = 0;

		for (let i = 0; i < 5; i++) {
			const d = dayjs().add(i, 'day');
			const key = d.format('YYYY-MM-DD');
			const dayEvents = allGroupedEvents[key] || [];

			if (dayEvents.length > 0) {
				// Day has events - add them up to the limit
				const remainingSlots = maxEvents - itemsCount;
				if (remainingSlots <= 0) {
					// Already reached limit, don't show this day
					break;
				}

				const eventsToShow = dayEvents.slice(0, remainingSlots);
				result.push({
					day: d.format('ddd'),
					date: parseInt(d.format('D')),
					month: d.format('MMM').toUpperCase(),
					events: eventsToShow
				});
				itemsCount += eventsToShow.length;

				if (itemsCount >= maxEvents) {
					// Reached limit, stop showing more days
					break;
				}
			} else {
				// Day has no events - count it as 1 item toward the limit
				if (itemsCount >= maxEvents) {
					// Already reached limit, don't show this day
					break;
				}

				result.push({
					day: d.format('ddd'),
					date: parseInt(d.format('D')),
					month: d.format('MMM').toUpperCase(),
					events: []
				});
				itemsCount += 1; // "No events" counts as 1

				if (itemsCount >= maxEvents) {
					// Reached limit, stop showing more days
					break;
				}
			}
		}

		this.cache = {
			data: result,
			expiry: Date.now() + TIMING_STRATEGIES.FREQUENT.interval,
			maxEvents,
			calendarColorsHash
		};
		return result;
	}
}

export async function GET(
	config: CalendarConfig,
	request?: Request
): Promise<
	Array<{ day: string; date: number; month: string; events: SimplifiedEvent[] }> | { error: string }
> {
	try {
		// Get origin from request URL
		const origin = request ? new URL(request.url).origin : 'http://localhost:5173';
		const data = await GoogleCalendarService.getEvents(origin, config);
		return data;
	} catch (error) {
		if ((error as Error).message === 'Not authenticated') {
			return { error: 'Not authenticated' };
		}
		console.error('Failed to fetch Google Calendar events', error);
		return { error: 'Failed to fetch events' };
	}
}
