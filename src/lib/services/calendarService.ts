import { dev } from '$app/environment';

export type CalendarEvent = {
	day: string;
	date: number;
	month: string;
	title: string;
	time: string;
	location?: string;
	isAllDay: boolean;
	category?: 'work' | 'personal';
};

export class CalendarService {
	static async getEvents(): Promise<CalendarEvent[][] | null> {
		try {
			if (dev) {
				return [
					[
						{
							day: 'Mon',
							date: 19,
							month: 'Aug',
							title: 'Team Stand-up',
							time: '9:00 AM',
							location: 'Office - Conference Room A',
							isAllDay: false,
							category: 'work'
						},
						{
							day: 'Mon',
							date: 19,
							month: 'Aug',
							title: 'Lunch with Jane',
							time: '12:30 PM',
							location: 'Cafe Bistro',
							isAllDay: false,
							category: 'personal'
						},
						{
							day: 'Mon',
							date: 19,
							month: 'Aug',
							title: 'Project Alpha Review',
							time: '2:00 PM',
							location: 'Zoom call',
							isAllDay: false,
							category: 'work'
						}
					],
					[
						{
							day: 'Tue',
							date: 20,
							month: 'Aug',
							title: 'All-Day Workshop',
							time: 'All Day',
							isAllDay: true,
							category: 'work'
						}
					],
					[
						{
							day: 'Wed',
							date: 21,
							month: 'Aug',
							title: 'Dentist Appointment',
							time: '10:00 AM',
							location: 'Downtown Dental',
							isAllDay: false,
							category: 'personal'
						},
						{
							day: 'Wed',
							date: 21,
							month: 'Aug',
							title: 'Groceries',
							time: '5:30 PM',
							location: 'SuperMart',
							isAllDay: false,
							category: 'personal'
						}
					],
					[
						{
							day: 'Thu',
							date: 22,
							month: 'Aug',
							title: 'Client Meeting',
							time: '1:00 PM',
							location: 'Client HQ',
							isAllDay: false,
							category: 'work'
						}
					],
					[
						{
							day: 'Fri',
							date: 23,
							month: 'Aug',
							title: 'Submit Expense Report',
							time: '3:00 PM',
							isAllDay: false,
							category: 'work'
						},
						{
							day: 'Fri',
							date: 23,
							month: 'Aug',
							title: 'Movie Night',
							time: '7:00 PM',
							isAllDay: false,
							category: 'personal'
						}
					]
				] as CalendarEvent[][];
			}
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
