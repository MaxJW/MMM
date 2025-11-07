import type { SpotifyTrack } from './types';
import { TIMING_STRATEGIES } from '$lib/core/timing';
import { getCached, setCache } from '$lib/core/utils';
import type { CacheEntry } from '$lib/core/utils';

let cache: CacheEntry<SpotifyTrack[]> | null = null;

interface SpotifyAccount {
	refreshToken: string;
	name?: string;
}

interface SpotifyConfig {
	clientId?: string;
	clientSecret?: string;
	accounts?: SpotifyAccount[];
}

async function getAccessToken(
	clientId: string,
	clientSecret: string,
	refreshToken: string
): Promise<string> {
	const res = await fetch('https://accounts.spotify.com/api/token', {
		method: 'POST',
		headers: {
			Authorization: 'Basic ' + Buffer.from(`${clientId}:${clientSecret}`).toString('base64'),
			'Content-Type': 'application/x-www-form-urlencoded'
		},
		body: new URLSearchParams({
			grant_type: 'refresh_token',
			refresh_token: refreshToken
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

async function fetchPlayerData(token: string): Promise<{
	track: SpotifyTrack | null;
	deviceName: string;
}> {
	// Fetch player state (includes item, device, and playback info)
	// Include additional_types=episode to get podcast episodes
	const playerRes = await fetch(
		'https://api.spotify.com/v1/me/player?additional_types=track,episode',
		{
			headers: { Authorization: `Bearer ${token}` }
		}
	);

	let track: SpotifyTrack | null = null;
	let deviceName = 'Unknown Device';

	// Handle no content or not found
	if (playerRes.status === 204 || playerRes.status === 404) {
		return { track: null, deviceName };
	}

	if (!playerRes.ok) {
		throw new Error(`HTTP ${playerRes.status}`);
	}

	const data = await playerRes.json();

	// Get device name from the same response
	if (data.device) {
		deviceName = data.device.name || 'Unknown Device';
	}

	if (!data.item) {
		return { track: null, deviceName };
	}

	// Handle both tracks and podcast episodes
	const isEpisode = data.item.type === 'episode';

	let artist: string;
	let albumArt: string;

	if (isEpisode) {
		// For podcast episodes: use show name as artist
		artist = data.item.show?.name || '';
		// Episodes can have their own images, fallback to show images
		albumArt = data.item.images?.[0]?.url || data.item.show?.images?.[0]?.url || '';
	} else {
		// For tracks: use artists array
		artist = data.item.artists?.map((a: { name: string }) => a.name).join(', ') || '';
		albumArt = data.item.album?.images?.[0]?.url || '';
	}

	track = {
		title: data.item.name || '',
		artist,
		albumArt,
		isPlaying: data.is_playing || false,
		progressMs: data.progress_ms || 0,
		durationMs: data.item.duration_ms || 0,
		deviceName,
		isEpisode
	};

	return { track, deviceName };
}

async function fetchTrackForAccount(
	clientId: string,
	clientSecret: string,
	account: SpotifyAccount
): Promise<SpotifyTrack | null> {
	const accountId = account.name || 'unknown';
	try {
		const token = await getAccessToken(clientId, clientSecret, account.refreshToken);
		const { track } = await fetchPlayerData(token);
		if (track) {
			// Add account name to track for identification
			track.accountName = account.name;
		}
		return track;
	} catch (error) {
		console.error(`[Spotify] Failed to fetch track for account "${accountId}":`, error);
		return null;
	}
}

export async function GET(config: SpotifyConfig): Promise<SpotifyTrack[] | { error: string }> {
	try {
		if (!config.clientId || !config.clientSecret) {
			throw new Error('Missing Spotify credentials');
		}

		// Check cache
		const cached = getCached(cache);
		if (cached) {
			return cached;
		}

		// Get accounts list
		if (!config.accounts || !Array.isArray(config.accounts)) {
			return [];
		}

		const accounts = config.accounts.filter((acc) => acc.refreshToken);

		if (accounts.length === 0) {
			return [];
		}

		// Fetch tracks for all accounts in parallel
		const trackPromises = accounts.map((account) =>
			fetchTrackForAccount(config.clientId as string, config.clientSecret as string, account)
		);

		const tracks = await Promise.all(trackPromises);

		// Filter to only tracks that are currently playing
		const playingTracks = tracks.filter(
			(track): track is SpotifyTrack => track !== null && track.isPlaying
		);

		// Cache for shorter interval to keep progress updated when playing
		const cacheDuration =
			playingTracks.length > 0
				? TIMING_STRATEGIES.VERY_FAST.interval // 10 seconds when playing for progress updates
				: TIMING_STRATEGIES.MEDIUM_FAST.interval; // 30 seconds when no tracks playing
		cache = setCache(cache, playingTracks, Date.now() + cacheDuration);
		return playingTracks;
	} catch (error) {
		console.error('Spotify API error:', error);
		return { error: error instanceof Error ? error.message : 'Failed to fetch Spotify tracks' };
	}
}
