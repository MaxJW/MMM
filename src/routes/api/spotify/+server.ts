import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getSpotifyConfig } from '$lib/config/userConfig';
import type { SpotifyTrack } from '$lib/types/spotify';
import { TIMING_STRATEGIES } from '$lib/types/util';

class SpotifyApi {
	private static cache: { data: SpotifyTrack; expiry: number } | null = null;

	private static async getAccessToken(): Promise<string> {
		const config = await getSpotifyConfig();
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
		const data = await res.json();
		return data.access_token;
	}

	private static async fetchTrack(): Promise<SpotifyTrack> {
		const token = await this.getAccessToken();
		const res = await fetch('https://api.spotify.com/v1/me/player/currently-playing', {
			headers: { Authorization: `Bearer ${token}` }
		});

		if (res.status === 204) return { title: '', artist: '', albumArt: '', isPlaying: false };
		if (!res.ok) throw new Error(`HTTP ${res.status}`);

		const data = await res.json();
		return {
			title: data.item.name,
			artist: data.item.artists.map((a: { name: string }) => a.name).join(', '),
			albumArt: data.item.album.images[0]?.url,
			isPlaying: data.is_playing
		};
	}

	static async getTrack(): Promise<SpotifyTrack> {
		if (this.cache && Date.now() < this.cache.expiry) return this.cache.data;
		const data = await this.fetchTrack();
		this.cache = { data, expiry: Date.now() + TIMING_STRATEGIES.FREQUENT.interval };
		return data;
	}
}

export const GET: RequestHandler = async () => {
	try {
		return json(await SpotifyApi.getTrack());
	} catch (err) {
		console.error('Spotify API error:', err);
		return json({ error: 'Failed to fetch Spotify track' }, { status: 500 });
	}
};
