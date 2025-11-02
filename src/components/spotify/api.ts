import type { SpotifyTrack } from './types';
import { TIMING_STRATEGIES } from '$lib/core/timing';
import { getCached, setCache } from '$lib/core/utils';
import type { CacheEntry } from '$lib/core/utils';

let cache: CacheEntry<SpotifyTrack> | null = null;

interface SpotifyConfig {
	clientId?: string;
	clientSecret?: string;
	refreshToken?: string;
}

async function getAccessToken(config: SpotifyConfig): Promise<string> {
	if (!config.clientId || !config.clientSecret || !config.refreshToken) {
		throw new Error('Missing Spotify credentials');
	}

	const res = await fetch('https://accounts.spotify.com/api/token', {
		method: 'POST',
		headers: {
			Authorization:
				'Basic ' + Buffer.from(`${config.clientId}:${config.clientSecret}`).toString('base64'),
			'Content-Type': 'application/x-www-form-urlencoded'
		},
		body: new URLSearchParams({
			grant_type: 'refresh_token',
			refresh_token: config.refreshToken
		})
	});

	if (!res.ok) {
		const errorData = await res.json().catch(() => ({}));
		throw new Error(
			errorData.error_description || `Failed to get access token: HTTP ${res.status}`
		);
	}

	const data = await res.json();
	return data.access_token;
}

async function fetchPlayerData(
	config: SpotifyConfig,
	token: string
): Promise<{
	track: SpotifyTrack | null;
	deviceName: string;
}> {
	// Fetch currently playing track and device info in parallel
	const [currentPlayingRes, playerRes] = await Promise.all([
		fetch('https://api.spotify.com/v1/me/player/currently-playing', {
			headers: { Authorization: `Bearer ${token}` }
		}),
		fetch('https://api.spotify.com/v1/me/player', {
			headers: { Authorization: `Bearer ${token}` }
		})
	]);

	let track: SpotifyTrack | null = null;
	let deviceName = 'Unknown Device';

	// Get device name
	if (playerRes.ok) {
		const playerData = await playerRes.json();
		if (playerData.device) {
			deviceName = playerData.device.name || 'Unknown Device';
		}
	}

	// Get currently playing track
	if (currentPlayingRes.status === 204) {
		return { track: null, deviceName };
	}

	if (!currentPlayingRes.ok) {
		if (currentPlayingRes.status === 404) {
			return { track: null, deviceName };
		}
		throw new Error(`HTTP ${currentPlayingRes.status}`);
	}

	const data = await currentPlayingRes.json();

	if (!data.item) {
		return { track: null, deviceName };
	}

	track = {
		title: data.item.name || '',
		artist: data.item.artists?.map((a: { name: string }) => a.name).join(', ') || '',
		albumArt: data.item.album?.images?.[0]?.url || '',
		isPlaying: data.is_playing || false,
		progressMs: data.progress_ms || 0,
		durationMs: data.item.duration_ms || 0,
		deviceName
	};

	return { track, deviceName };
}

async function fetchTrack(config: SpotifyConfig): Promise<SpotifyTrack | null> {
	const token = await getAccessToken(config);
	const { track } = await fetchPlayerData(config, token);
	return track;
}

export async function GET(config: SpotifyConfig): Promise<SpotifyTrack | { error: string }> {
	try {
		// Check cache, but allow shorter cache for progress updates
		const cached = getCached(cache);
		if (cached) {
			return cached;
		}

		const data = await fetchTrack(config);

		// If no track is playing, return a not-playing state
		if (!data) {
			const emptyTrack: SpotifyTrack = {
				title: '',
				artist: '',
				albumArt: '',
				isPlaying: false,
				progressMs: 0,
				durationMs: 0,
				deviceName: 'No active device'
			};
			// Cache empty state for UI.FADE interval (10 seconds)
			cache = setCache(cache, emptyTrack, Date.now() + TIMING_STRATEGIES.UI.FADE);
			return emptyTrack;
		}

		// Cache for shorter interval to keep progress updated when playing, longer when paused
		const cacheDuration = data.isPlaying
			? TIMING_STRATEGIES.UI.FADE // 10 seconds when playing for progress updates
			: TIMING_STRATEGIES.FREQUENT.interval; // 5 minutes when paused
		cache = setCache(cache, data, Date.now() + cacheDuration);
		return data;
	} catch (error) {
		console.error('Spotify API error:', error);
		return { error: error instanceof Error ? error.message : 'Failed to fetch Spotify track' };
	}
}
