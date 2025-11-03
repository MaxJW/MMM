// Static imports for all config components
// This ensures they're available at runtime in both dev and production builds
// Components can optionally provide a config.svelte file for custom configuration UI

import SpotifyConfig from './spotify/config.svelte';

// Map component IDs to their config components
// Only components that have custom config UIs are included here
export const configComponentMap: Record<string, unknown> = {
	spotify: SpotifyConfig
};
