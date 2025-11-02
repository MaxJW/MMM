import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import type { EventConfig } from '$lib/config/events';
import { generateQRCode } from '$lib/core/utils';

// Import images from the component assets folder at build time
// Vite bundles all matching images at build time
const eventImageModules = import.meta.glob('./assets/*.png', {
	eager: true,
	import: 'default'
}) as Record<string, string>;

function resolveEventImage(slug: string | undefined): string | undefined {
	if (!slug) return undefined;

	// Direct lookup using the slug in the path
	// Vite processes paths like './assets/halloween.png' -> '/_app/assets/halloween-<hash>.png'
	const imagePath = `./assets/${slug}.png`;
	return eventImageModules[imagePath];
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

interface EventsConfig {
	events?: Array<Omit<EventConfig, 'eventImage' | 'qrCode'>>;
}

export async function GET(
	config: EventsConfig,
	request?: Request
): Promise<
	| { currentEvent: null; greeting: null }
	| { currentEvent: EventConfig & { startDate: string; endDate: string }; greeting: string | null }
	| { events: Array<Omit<EventConfig, 'eventImage' | 'qrCode'>> }
	| { error: string }
> {
	try {
		const eventsConfig = config.events || [];

		// If all=true is requested, return all events (for settings page)
		const url = request ? new URL(request.url) : null;
		if (url?.searchParams.get('all') === 'true') {
			return { events: eventsConfig };
		}

		const now = dayjs();
		const today = now.startOf('day');

		// Find current event
		let currentEvent: (EventConfig & { startDate: Dayjs; endDate: Dayjs }) | null = null;
		let greeting: string | null = null;

		for (const event of eventsConfig) {
			// Resolve image and QR code
			const eventImage = resolveEventImage(event.eventSlug);
			const qrCode = event.qrCodeLink ? await generateQRCode(event.qrCodeLink) : undefined;

			const fullEvent: EventConfig = {
				...event,
				eventImage,
				qrCode
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

		if (currentEvent) {
			return {
				currentEvent: {
					...currentEvent,
					startDate: currentEvent.startDate.toISOString(),
					endDate: currentEvent.endDate.toISOString()
				},
				greeting
			};
		}

		return {
			currentEvent: null,
			greeting: null
		};
	} catch (error) {
		console.error('Error processing events:', error);
		return { error: 'Failed to process events' };
	}
}
