import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { broadcastConfigChange } from '$lib/services/configStream';
import { readFile, writeFile, mkdir } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import type { EventConfig } from '$lib/config/events';
import EventImgHalloween from '$lib/assets/events/halloween/EventImage.png';
import QRCodeHalloween from '$lib/assets/events/halloween/QRCode.png';

const EVENTS_FILE = join(process.cwd(), 'data', 'events.json');
const EVENTS_DIR = join(process.cwd(), 'data');

/**
 * Ensure events directory exists
 */
async function ensureEventsDir() {
	if (!existsSync(EVENTS_DIR)) {
		await mkdir(EVENTS_DIR, { recursive: true });
	}
}

interface EventSlugAssets {
	eventImage: string;
	qrCode: string;
}

const assetMap: Record<string, EventSlugAssets> = {
	halloween: {
		eventImage: EventImgHalloween,
		qrCode: QRCodeHalloween
	}
};

function resolveEventAssets(slug: string | undefined): Partial<EventSlugAssets> {
	if (!slug || !assetMap[slug]) {
		return {};
	}
	return assetMap[slug];
}

function getEventDates(event: EventConfig, now = dayjs()) {
	const year = now.year();

	let startDate = dayjs(`${year}-${event.start} 00:00:00`);
	let endDate = dayjs(`${year}-${event.end} 23:59:59`);

	if (endDate.isBefore(startDate)) {
		endDate = endDate.add(1, 'year');
	}

	if (endDate.isBefore(now)) {
		startDate = startDate.add(1, 'year');
		endDate = endDate.add(1, 'year');
	}

	return { startDate, endDate };
}

export const GET: RequestHandler = async ({ url }) => {
	try {
		if (!existsSync(EVENTS_FILE)) {
			// If all=true is requested, return empty array
			if (url.searchParams.get('all') === 'true') {
				return json({ events: [] });
			}
			return json({ currentEvent: null, greeting: null });
		}

		const data = await readFile(EVENTS_FILE, 'utf8');
		const eventsConfig = JSON.parse(data) as Array<
			Omit<EventConfig, 'eventImage' | 'qrCode'> & { eventSlug?: string }
		>;

		// Return all events if requested (for settings page)
		if (url.searchParams.get('all') === 'true') {
			return json({ events: eventsConfig });
		}

		const now = dayjs();
		const today = now.startOf('day');

		// Find current event
		let currentEvent: (EventConfig & { startDate: Dayjs; endDate: Dayjs }) | null = null;
		let greeting: string | null = null;

		for (const event of eventsConfig) {
			const assets = resolveEventAssets(event.eventSlug);
			const fullEvent: EventConfig = {
				...event,
				eventImage: assets.eventImage,
				qrCode: assets.qrCode
			};

			const { startDate, endDate } = getEventDates(fullEvent, now);
			if (
				(today.isAfter(startDate) || today.isSame(startDate)) &&
				(today.isBefore(endDate) || today.isSame(endDate))
			) {
				currentEvent = { ...fullEvent, startDate, endDate };
				greeting = currentEvent.customGreeting || null;
				break;
			}
		}

		return json({
			currentEvent: currentEvent
				? {
						...currentEvent,
						startDate: currentEvent.startDate.toISOString(),
						endDate: currentEvent.endDate.toISOString()
					}
				: null,
			greeting
		});
	} catch (error) {
		console.error('Error loading events:', error);
		// Return appropriate empty response based on request
		if (url.searchParams.get('all') === 'true') {
			return json({ events: [] });
		}
		return json({ currentEvent: null, greeting: null });
	}
};

export const PUT: RequestHandler = async ({ request }) => {
	try {
		const body = await request.json();
		const events = body.events as Array<Omit<EventConfig, 'eventImage' | 'qrCode'>>;

		// Ensure data directory exists
		await ensureEventsDir();

		// Validate events structure
		for (const event of events) {
			if (!event.start || !event.end) {
				return json({ error: 'Each event must have start and end dates' }, { status: 400 });
			}
			// Validate date format (MM-DD)
			if (!/^\d{2}-\d{2}$/.test(event.start) || !/^\d{2}-\d{2}$/.test(event.end)) {
				return json({ error: 'Dates must be in MM-DD format (e.g., 10-31)' }, { status: 400 });
			}
		}

		await writeFile(EVENTS_FILE, JSON.stringify(events, null, 2));

		// Broadcast config change to all connected clients
		broadcastConfigChange();

		return json({ success: true, events });
	} catch (error) {
		console.error('Error saving events:', error);
		return json({ error: 'Failed to save events' }, { status: 500 });
	}
};
