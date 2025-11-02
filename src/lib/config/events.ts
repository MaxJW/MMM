import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';

export interface EventConfig {
	start: string; // Format: 'MM-DD'
	end: string; // Format: 'MM-DD'
	eventSlug?: string; // Folder name in assets/events/ (e.g., 'halloween')
	eventText?: string;
	customGreeting?: string;
	// Resolved asset paths (set internally, not in JSON)
	eventImage?: string;
	qrCode?: string;
}

/**
 * Get the currently active recurring event (client-side)
 * Fetches from API endpoint
 */
export async function getCurrentEvent(): Promise<
	(EventConfig & { startDate: Dayjs; endDate: Dayjs }) | null
> {
	try {
		const response = await fetch('/api/events');
		if (!response.ok) return null;

		const data = await response.json();
		if (!data.currentEvent) return null;

		return {
			...data.currentEvent,
			startDate: dayjs(data.currentEvent.startDate),
			endDate: dayjs(data.currentEvent.endDate)
		};
	} catch (error) {
		console.error('Failed to fetch current event:', error);
		return null;
	}
}

/**
 * Get the custom greeting for the current event, if any (client-side)
 */
export async function getCurrentEventGreeting(): Promise<string | null> {
	try {
		const response = await fetch('/api/events');
		if (!response.ok) return null;

		const data = await response.json();
		return data.greeting || null;
	} catch (error) {
		console.error('Failed to fetch event greeting:', error);
		return null;
	}
}
