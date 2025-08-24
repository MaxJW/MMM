import type { SpotifyTrack } from '$lib/types/spotify';

export class SpotifyService {
	static async getCurrentTrack(): Promise<SpotifyTrack | null> {
		try {
			const res = await fetch('/api/spotify');
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			return (await res.json()) as SpotifyTrack;
		} catch (err) {
			console.error('Failed to fetch Spotify track:', err);
			return null;
		}
	}
}
