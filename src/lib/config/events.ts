import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import eventsConfig from './events.json';

// Import known assets for mapping by slug
import EventImgHalloween from '$lib/assets/events/halloween/EventImage.png';
import QRCodeHalloween from '$lib/assets/events/halloween/QRCode.png';

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

interface EventSlugAssets {
	eventImage: string;
	qrCode: string;
}

// Map of event slugs to their imported assets
const assetMap: Record<string, EventSlugAssets> = {
	halloween: {
		eventImage: EventImgHalloween,
		qrCode: QRCodeHalloween
	}
};

/**
 * Resolve assets for an event slug.
 * Looks up the slug in the asset map and returns the imported asset URLs.
 */
function resolveEventAssets(slug: string | undefined): Partial<EventSlugAssets> {
	if (!slug || !assetMap[slug]) {
		return {};
	}
	return assetMap[slug];
}

// Load and process events from JSON config
const recurringEvents: EventConfig[] = (eventsConfig as EventConfig[]).map(
	(event: Omit<EventConfig, 'eventImage' | 'qrCode'> & { eventSlug?: string }) => {
		const assets = resolveEventAssets(event.eventSlug);
		return {
			...event,
			eventImage: assets.eventImage,
			qrCode: assets.qrCode
		};
	}
);

/**
 * Given a recurring event with start/end dates (MM-DD),
 * return its start and end as Dayjs objects for the correct year.
 */
function getEventDates(event: EventConfig, now = dayjs()) {
	const year = now.year();

	let startDate = dayjs(`${year}-${event.start} 00:00:00`);
	let endDate = dayjs(`${year}-${event.end} 23:59:59`);

	// If the event spans across years (e.g., Dec 31 – Jan 1)
	if (endDate.isBefore(startDate)) {
		endDate = endDate.add(1, 'year');
	}

	// If event has already passed this year, roll it over to next year
	if (endDate.isBefore(now)) {
		startDate = startDate.add(1, 'year');
		endDate = endDate.add(1, 'year');
	}

	return { startDate, endDate };
}

/**
 * Get the currently active recurring event
 */
export function getCurrentEvent(): (EventConfig & { startDate: Dayjs; endDate: Dayjs }) | null {
	const now = dayjs();
	const today = now.startOf('day');

	for (const event of recurringEvents) {
		const { startDate, endDate } = getEventDates(event, now);
		if (
			(today.isAfter(startDate) || today.isSame(startDate)) &&
			(today.isBefore(endDate) || today.isSame(endDate))
		) {
			return { ...event, startDate, endDate };
		}
	}

	return null;
}

/**
 * Get the custom greeting for the current event, if any
 */
export function getCurrentEventGreeting(): string | null {
	const currentEvent = getCurrentEvent();
	return currentEvent?.customGreeting || null;
}
