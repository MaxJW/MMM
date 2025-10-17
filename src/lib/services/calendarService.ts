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
				return [
					{
						day: 'Mon',
						date: 19,
						month: 'AUG',
						events: [
							{
								title: 'Team Stand-up',
								time: '9:00 AM',
								location: 'Office - Conference Room A',
								isAllDay: false,
								category: 'work'
							},
							{
								title: 'Lunch with Jane',
								time: '12:30 PM',
								location: 'Cafe Bistro',
								isAllDay: false,
								category: 'personal'
							}
						]
					},
					{ day: 'Tue', date: 20, month: 'AUG', events: [] },
					{
						day: 'Wed',
						date: 21,
						month: 'AUG',
						events: [
							{
								title: 'Dentist Appointment',
								time: '10:00 AM',
								location: 'Downtown Dental',
								isAllDay: false,
								category: 'personal'
							}
						]
					},
					{ day: 'Thu', date: 22, month: 'AUG', events: [] },
					{
						day: 'Fri',
						date: 23,
						month: 'AUG',
						events: [
							{ title: 'Submit Expense Report', time: '3:00 PM', isAllDay: false, category: 'work' }
						]
					}
				];
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
