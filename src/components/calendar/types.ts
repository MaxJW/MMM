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
