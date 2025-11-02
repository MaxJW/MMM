// Store all connected SSE clients
const clients = new Set<ReadableStreamDefaultController>();

/**
 * Broadcast a config change notification to all connected clients
 */
export function broadcastConfigChange() {
	const message = `data: ${JSON.stringify({ type: 'config-changed', timestamp: Date.now() })}\n\n`;

	// Send to all connected clients
	for (const client of clients) {
		try {
			client.enqueue(new TextEncoder().encode(message));
		} catch {
			// Client disconnected, remove from set
			clients.delete(client);
		}
	}
}

/**
 * Add a client to the SSE stream
 */
export function addClient(controller: ReadableStreamDefaultController) {
	clients.add(controller);
}

/**
 * Remove a client from the SSE stream
 */
export function removeClient(controller: ReadableStreamDefaultController) {
	clients.delete(controller);
}
