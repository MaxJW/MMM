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
			console.log(
				`[Spotify] Account "${accountId}": Track found - ${track.title} by ${track.artist}, isPlaying: ${track.isPlaying}`
			);
		} else {
			console.log(`[Spotify] Account "${accountId}": No track currently playing`);
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

		// Log results for debugging
		console.log(`[Spotify] Fetched tracks for ${accounts.length} accounts:`, {
			totalAccounts: accounts.length,
			tracksFetched: tracks.length,
			nullTracks: tracks.filter((t) => t === null).length,
			playingTracks: tracks.filter((t) => t !== null && t.isPlaying).length,
			accountNames: accounts.map((a) => a.name || 'unknown')
		});

		// Filter to only tracks that are currently playing
		const playingTracks = tracks.filter(
			(track): track is SpotifyTrack => track !== null && track.isPlaying
		);

		// Cache for shorter interval to keep progress updated when playing
		const cacheDuration =
			playingTracks.length > 0
				? TIMING_STRATEGIES.UI.FADE // 10 seconds when playing for progress updates
				: TIMING_STRATEGIES.FREQUENT.interval; // 5 minutes when no tracks playing
		cache = setCache(cache, playingTracks, Date.now() + cacheDuration);
		return playingTracks;
	} catch (error) {
		console.error('Spotify API error:', error);
		return { error: error instanceof Error ? error.message : 'Failed to fetch Spotify tracks' };
	}
}
