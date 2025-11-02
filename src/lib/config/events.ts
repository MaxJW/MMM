import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';

export interface EventConfig {
	start: string; // Format: 'MM-DD'
	end: string; // Format: 'MM-DD'
	eventSlug?: string; // Image filename in src/components/events/assets/ (e.g., 'halloween' for halloween.png)
	eventText?: string;
	customGreeting?: string;
	qrCodeLink?: string; // URL or text to encode in QR code
	// Resolved asset paths (set internally, not in JSON)
	eventImage?: string;
	qrCode?: string; // Generated QR code data URL
}

/**
 * Get the currently active recurring event (client-side)
 * Fetches from component API endpoint
 */
export async function getCurrentEvent(): Promise<
	(EventConfig & { startDate: Dayjs; endDate: Dayjs }) | null
> {
	try {
		const response = await fetch('/api/components/events');
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
		const response = await fetch('/api/components/events');
		if (!response.ok) return null;

		const data = await response.json();
		return data.greeting || null;
	} catch (error) {
		console.error('Failed to fetch event greeting:', error);
		return null;
	}
}
