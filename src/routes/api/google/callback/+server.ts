import type { RequestHandler } from './$types';
import { env } from '$env/dynamic/private';
import { google } from 'googleapis';
import { dev } from '$app/environment';

export const GET: RequestHandler = async ({ url, cookies }) => {
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
		const accessToken = tokens.access_token ?? '';
		const refreshToken = tokens.refresh_token ?? '';
		const expiresIn = tokens.expiry_date
			? Math.max(0, Math.floor((tokens.expiry_date - Date.now()) / 1000))
			: 3600;

		if (refreshToken) {
			cookies.set('gc_refresh_token', refreshToken, {
				path: '/',
				httpOnly: true,
				secure: !dev,
				sameSite: 'lax',
				maxAge: 60 * 60 * 24 * 365
			});
		}

		if (accessToken) {
			cookies.set('gc_access_token', accessToken, {
				path: '/',
				httpOnly: true,
				secure: !dev,
				sameSite: 'lax',
				maxAge: expiresIn
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
