import { dev } from '$app/environment';

export type CalendarDay = {
	day: string;
	date: number;
	month: string;
	events: CalendarEvent[];
};

export type CalendarEvent = {
	title: string;
	time: string;
	location?: string;
	isAllDay: boolean;
	category?: 'work' | 'personal';
};

export class CalendarService {
	static async getEvents(): Promise<CalendarDay[] | null> {
		try {
			if (dev) {
				// Return empty calendar in dev mode - use real Google Calendar for testing
				return [];
			}
			const res = await fetch('/api/google/events');
			if (res.status === 401) return null;
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			return (await res.json()) as CalendarDay[];
		} catch (err) {
			console.error('Failed to fetch calendar events:', err);
			return null;
		}
	}
}
