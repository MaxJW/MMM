export type CalendarEvent = {
	day: string;
	date: number;
	month: string;
	title: string;
	time: string;
	isAllDay: boolean;
	category?: 'work' | 'personal';
};

export class CalendarService {
	static async getEvents(): Promise<CalendarEvent[][] | null> {
		try {
			const res = await fetch('/api/google/events');
			if (res.status === 401) return null;
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			return (await res.json()) as CalendarEvent[][];
		} catch (err) {
			console.error('Failed to fetch calendar events:', err);
			return null;
		}
	}
}
