import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';

import EventImgHalloween from '$lib/assets/events/halloween/EventImage.png';
import QRCodeHalloween from '$lib/assets/events/halloween/QRCode.png';

export interface EventConfig {
	startDate: Dayjs;
	endDate: Dayjs;
	eventText: string;
	eventImage: string;
	qrCode: string;
	customGreeting?: string;
}

const events: EventConfig[] = [
	{
		startDate: dayjs('2025-10-31 00:00:00'),
		endDate: dayjs('2025-11-01 23:59:59'),
		eventText: 'Join the Halloween Photo Challenge below!',
		eventImage: EventImgHalloween,
		qrCode: QRCodeHalloween,
		customGreeting: 'Happy Halloween'
	}
	// Add future events here:
	// {
	// 	startDate: dayjs('2024-12-24'),
	// 	endDate: dayjs('2024-12-26'),
	// 	eventText: 'Merry Christmas!',
	// 	eventImage: EventImgChristmas,
	// 	qrCode: QRCodeChristmas,
	// 	customGreeting: 'Merry Christmas!'
	// }
];

/**
 * Get the currently active event based on the current date
 * @returns The active event config or null if no event is active
 */
export function getCurrentEvent(): EventConfig | null {
	const now = dayjs();
	const today = now.startOf('day');

	return (
		events.find((event) => {
			const start = event.startDate.startOf('day');
			const end = event.endDate.startOf('day');
			return (
				(today.isAfter(start) || today.isSame(start)) && (today.isBefore(end) || today.isSame(end))
			);
		}) || null
	);
}

/**
 * Get the custom greeting for the current event, if any
 * @returns Custom greeting string or null
 */
export function getCurrentEventGreeting(): string | null {
	const currentEvent = getCurrentEvent();
	return currentEvent?.customGreeting || null;
}
