import QRCode from 'qrcode';

/**
 * Shared utility functions for the mirror application
 */

/**
 * Simple cache helper for API responses
 */
export interface CacheEntry<T> {
	data: T;
	expiry: number;
}

export function getCached<T>(cache: CacheEntry<T> | null): T | null {
	if (cache && Date.now() < cache.expiry) {
		return cache.data;
	}
	return null;
}

export function setCache<T>(cache: CacheEntry<T> | null, data: T, expiry: number): CacheEntry<T> {
	return { data, expiry };
}

/**
 * Error handling helper
 */
export function handleApiError(error: unknown, context: string): { error: string } {
	console.error(`Error in ${context}:`, error);
	return {
		error: error instanceof Error ? error.message : 'An unknown error occurred'
	};
}

/**
 * Generate a QR code as a base64 data URL
 * Creates a white QR code on a transparent background (optimized for dark themes)
 *
 * @param text - The text or URL to encode in the QR code
 * @param options - Optional QR code generation options
 * @returns Base64 data URL of the QR code image, or undefined if generation fails
 */
export async function generateQRCode(
	text: string,
	options?: {
		width?: number;
		margin?: number;
	}
): Promise<string | undefined> {
	if (!text) return undefined;

	try {
		// Generate QR code as PNG buffer
		// White QR code on transparent background (optimized for dark themes)
		const qrCodeBuffer = await QRCode.toBuffer(text, {
			width: options?.width ?? 800,
			margin: options?.margin ?? 2,
			color: {
				dark: '#FFFFFF', // White QR code pattern
				light: '#00000000' // Transparent background (RGBA with alpha 0)
			}
		});

		// Convert buffer to base64 data URL
		const base64 = qrCodeBuffer.toString('base64');
		return `data:image/png;base64,${base64}`;
	} catch (error) {
		console.error('Error generating QR code:', error);
		return undefined;
	}
}
