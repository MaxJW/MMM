import type { RequestHandler } from './$types';
import { getComponentConfig } from '$lib/core/config';

export const GET: RequestHandler = async (event) => {
	const { url } = event;
	const config = await getComponentConfig('spotify');

	if (!config.clientId || !config.clientSecret) {
		return new Response('Spotify OAuth not configured', { status: 500 });
	}

	const redirectUri = `${url.origin}/api/spotify/callback`;
	const clientId = config.clientId as string;
	const scopes = ['user-read-currently-playing', 'user-read-playback-state'];
	const scope = scopes.join(' ');

	// Spotify OAuth URL
	const authUrl = new URL('https://accounts.spotify.com/authorize');
	authUrl.searchParams.set('client_id', clientId);
	authUrl.searchParams.set('response_type', 'code');
	authUrl.searchParams.set('redirect_uri', redirectUri);
	authUrl.searchParams.set('scope', scope);
	authUrl.searchParams.set('show_dialog', 'true'); // Force re-authentication to get refresh token

	return new Response(null, {
		status: 302,
		headers: { Location: authUrl.toString() }
	});
};
