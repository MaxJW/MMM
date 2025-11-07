// Time constants in milliseconds
export const THIRTY_MIN = 30 * 60 * 1000;
export const SIXTY_MIN = 60 * 60 * 1000;
export const FIVE_MIN = 5 * 60 * 1000;
export const TEN_MIN = 10 * 60 * 1000;
export const FIFTEEN_MIN = 15 * 60 * 1000;

// Timing strategies for different data types
export const TIMING_STRATEGIES = {
	// High-frequency updates (real-time data)
	REALTIME: {
		interval: 1000, // 1 second
		description: 'Real-time updates (clock, seconds)'
	},

	VERY_FAST: {
		interval: 5_000, // 5 seconds
		description: 'Very fast updates (spotify playing)'
	},

	FAST: {
		interval: 10_000, // 10 seconds
		description: 'Fast updates (spotify playing)'
	},

	MEDIUM_FAST: {
		interval: 30_000, // 30 seconds
		description: 'Medium-fast updates (spotify playing)'
	},

	MEDIUM: {
		interval: 60_000, // 1 minute
		description: 'Medium-frequency updates'
	},

	// Medium-frequency updates (time-sensitive data)
	FREQUENT: {
		interval: FIVE_MIN, // 5 minutes
		description: 'Frequent updates (system stats, weather)'
	},

	// Standard updates (most data)
	STANDARD: {
		interval: THIRTY_MIN, // 30 minutes
		description: 'Standard updates (RSS, weather forecasts)'
	},

	// Low-frequency updates (rarely changing data)
	INFREQUENT: {
		interval: SIXTY_MIN, // 60 minutes
		description: 'Infrequent updates (bin collections, static data)'
	},

	// UI-specific intervals
	UI: {
		MINUTE: 60_000, // 1 minute
		FADE: 10_000, // 10 seconds (for RSS rotation)
		description: 'UI-specific intervals'
	}
} as const;
