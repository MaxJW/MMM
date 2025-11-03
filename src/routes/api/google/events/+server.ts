import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getGoogleConfig } from '$lib/config/userConfig';
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
	category?: 'work' | 'personal';
};

type CalendarListEntryLite = {
	id?: string;
	deleted?: boolean;
	selected?: boolean;
	primary?: boolean;
};

type GoogleEventLite = {
	summary?: string;
	start?: { dateTime?: string; date?: string };
	end?: { dateTime?: string; date?: string };
	location?: string;
};

class GoogleCalendarService {
	private static cache: {
		data: Array<{ day: string; date: number; month: string; events: SimplifiedEvent[] }>;
		expiry: number;
	} | null = null;

	static async getEvents(
		origin: string
	): Promise<Array<{ day: string; date: number; month: string; events: SimplifiedEvent[] }>> {
		if (this.cache && Date.now() < this.cache.expiry) {
			return this.cache.data;
		}

		// Load tokens from file
		const tokenData = await TokenStorage.loadTokens();
		if (!tokenData) {
			throw new Error('Not authenticated');
		}

		const config = await getGoogleConfig();
		if (!config.clientId || !config.clientSecret) {
			throw new Error('Google OAuth not configured');
		}

		// Get maxEvents from config (default to 12)
		const maxEvents = typeof config.maxEvents === 'number' ? config.maxEvents : 12;

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

		// Convert events to simplified format and limit to maxEvents
		const simplifiedEvents: Array<{ simplified: SimplifiedEvent; key: string }> = [];
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
			const simplified: SimplifiedEvent = {
				day: d.format('ddd'),
				date: parseInt(d.format('D')),
				month,
				title: event.summary || '(No title)',
				time,
				location: event.location ? event.location.split(',')[0] : undefined,
				isAllDay,
				category: cal.primary ? 'personal' : 'work'
			};
			simplifiedEvents.push({ simplified, key });

			// Stop once we reach maxEvents
			if (simplifiedEvents.length >= maxEvents) {
				break;
			}
		}

		// Group simplified events by day
		const grouped: Record<string, SimplifiedEvent[]> = {};
		for (const { simplified, key } of simplifiedEvents) {
			if (!grouped[key]) grouped[key] = [];
			grouped[key].push(simplified);
		}

		const days: Array<{ day: string; date: number; month: string; key: string }> = [];
		for (let i = 0; i < 5; i++) {
			const d = dayjs().add(i, 'day');
			days.push({
				day: d.format('ddd'),
				date: parseInt(d.format('D')),
				month: d.format('MMM').toUpperCase(),
				key: d.format('YYYY-MM-DD')
			});
		}

		const result: Array<{ day: string; date: number; month: string; events: SimplifiedEvent[] }> =
			days.map(({ day, date, month, key }) => ({
				day,
				date,
				month,
				events: grouped[key] || []
			}));

		this.cache = { data: result, expiry: Date.now() + TIMING_STRATEGIES.FREQUENT.interval };
		return result;
	}
}

export const GET: RequestHandler = async ({ url }) => {
	try {
		const data = await GoogleCalendarService.getEvents(url.origin);
		return json(data);
	} catch (error) {
		if ((error as Error).message === 'Not authenticated') {
			return json({ error: 'Not authenticated' }, { status: 401 });
		}
		console.error('Failed to fetch Google Calendar events', error);
		return json({ error: 'Failed to fetch events' }, { status: 500 });
	}
};
