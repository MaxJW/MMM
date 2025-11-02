import type { RequestHandler } from './$types';
import { addClient, removeClient } from '$lib/services/configStream';

/**
 * SSE endpoint - clients connect here to receive config change notifications
 */
export const GET: RequestHandler = async ({ request }) => {
	const stream = new ReadableStream({
		start(controller) {
			// Add client to the set
			addClient(controller);

			// Send initial connection message
			const initMessage = `data: ${JSON.stringify({ type: 'connected', timestamp: Date.now() })}\n\n`;
			controller.enqueue(new TextEncoder().encode(initMessage));

			// Handle client disconnect via abort signal
			request.signal.addEventListener('abort', () => {
				removeClient(controller);
				try {
					controller.close();
				} catch {
					// Already closed
				}
			});
		},
		cancel(controller) {
			// Client disconnected - remove from set
			removeClient(controller);
		}
	});

	return new Response(stream, {
		headers: {
			'Content-Type': 'text/event-stream',
			'Cache-Control': 'no-cache',
			Connection: 'keep-alive'
		}
	});
};
