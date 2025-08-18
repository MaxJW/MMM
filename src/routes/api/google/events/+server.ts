import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { env } from '$env/dynamic/private';
import { google } from 'googleapis';
import dayjs from 'dayjs';
import { TIMING_STRATEGIES } from '$lib/types/util';

type SimplifiedEvent = {
	day: string;
	date: number;
	month: string;
	title: string;
	time: string;
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
};

class GoogleCalendarService {
	private static cache: { data: SimplifiedEvent[][]; expiry: number } | null = null;

	static async getEvents(
		accessToken: string | undefined,
		refreshToken: string | undefined,
		origin: string
	): Promise<SimplifiedEvent[][]> {
		if (this.cache && Date.now() < this.cache.expiry) {
			return this.cache.data;
		}

		const GOOGLE_CLIENT_ID = env.GOOGLE_CLIENT_ID;
		const GOOGLE_CLIENT_SECRET = env.GOOGLE_CLIENT_SECRET;
		if (!GOOGLE_CLIENT_ID || !GOOGLE_CLIENT_SECRET) {
			throw new Error('Missing Google credentials');
		}

		if (!accessToken && !refreshToken) {
			throw new Error('Not authenticated');
		}

		const oauth2Client = new google.auth.OAuth2({
			clientId: GOOGLE_CLIENT_ID,
			clientSecret: GOOGLE_CLIENT_SECRET,
			redirectUri: `${origin}/api/google/callback`
		});

		if (accessToken) oauth2Client.setCredentials({ access_token: accessToken });
		if (refreshToken) oauth2Client.setCredentials({ refresh_token: refreshToken });

		await oauth2Client.getAccessToken().catch((err: unknown) => {
			console.error('Failed to refresh Google access token', err);
		});

		const calendar = google.calendar({ version: 'v3', auth: oauth2Client });

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
					orderBy: 'startTime'
				});
				const eventsUnknown = (res.data.items || []) as unknown[];
				return eventsUnknown.map((e) => ({ event: e as GoogleEventLite, calendar: cal }));
			})
		);

		const grouped: Record<string, SimplifiedEvent[]> = {};
		for (const { event, calendar: cal } of allEvents.flat() as Array<{
			event: GoogleEventLite;
			calendar: CalendarListEntryLite;
		}>) {
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
				isAllDay,
				category: cal.primary ? 'personal' : 'work'
			};
			if (!grouped[key]) grouped[key] = [];
			grouped[key].push(simplified);
		}

		const orderedKeys = Object.keys(grouped).sort();
		const result = orderedKeys.map((k) => grouped[k]);

		this.cache = { data: result, expiry: Date.now() + TIMING_STRATEGIES.FREQUENT.interval };
		return result;
	}
}

export const GET: RequestHandler = async ({ cookies, url }) => {
	try {
		const accessToken = cookies.get('gc_access_token');
		const refreshToken = cookies.get('gc_refresh_token');
		const data = await GoogleCalendarService.getEvents(accessToken, refreshToken, url.origin);
		return json(data);
	} catch (error) {
		if ((error as Error).message === 'Not authenticated') {
			return json({ error: 'Not authenticated' }, { status: 401 });
		}
		console.error('Failed to fetch Google Calendar events', error);
		return json({ error: 'Failed to fetch events' }, { status: 500 });
	}
};
