import type { RequestHandler } from './$types';
import { env } from '$env/dynamic/private';
import { google } from 'googleapis';
import { TokenStorage } from '$lib/services/tokenStorage';

export const GET: RequestHandler = async ({ url }) => {
	const code = url.searchParams.get('code');
	if (!code) {
		return new Response('Missing code', { status: 400 });
	}

	const GOOGLE_CLIENT_ID = env.GOOGLE_CLIENT_ID;
	const GOOGLE_CLIENT_SECRET = env.GOOGLE_CLIENT_SECRET;
	if (!GOOGLE_CLIENT_ID || !GOOGLE_CLIENT_SECRET) {
		return new Response('Missing Google OAuth credentials', { status: 500 });
	}

	const redirectUri = `${url.origin}/api/google/callback`;
	const oauth2Client = new google.auth.OAuth2({
		clientId: GOOGLE_CLIENT_ID,
		clientSecret: GOOGLE_CLIENT_SECRET,
		redirectUri
	});

	try {
		const { tokens } = await oauth2Client.getToken(code);

		// Save tokens to file instead of cookies
		if (tokens.refresh_token) {
			await TokenStorage.saveTokens({
				refreshToken: tokens.refresh_token,
				accessToken: tokens.access_token || undefined,
				expiryDate: tokens.expiry_date || undefined
			});
		}

		return new Response(null, {
			status: 302,
			headers: { Location: '/' }
		});
	} catch (err) {
		console.error('OAuth callback failed', err);
		return new Response('Failed to exchange code', { status: 500 });
	}
};
