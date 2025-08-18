import type { RequestHandler } from './$types';
import { env } from '$env/dynamic/private';
import { google } from 'googleapis';

export const GET: RequestHandler = async (event) => {
	const { url } = event;
	const GOOGLE_CLIENT_ID = env.GOOGLE_CLIENT_ID;
	const GOOGLE_CLIENT_SECRET = env.GOOGLE_CLIENT_SECRET;
	if (!GOOGLE_CLIENT_ID || !GOOGLE_CLIENT_SECRET) {
		return new Response(JSON.stringify({ error: 'Missing Google OAuth credentials' }), {
			status: 500,
			headers: { 'content-type': 'application/json' }
		});
	}

	const redirectUri = `${url.origin}/api/google/callback`;

	const oauth2Client = new google.auth.OAuth2({
		clientId: GOOGLE_CLIENT_ID,
		clientSecret: GOOGLE_CLIENT_SECRET,
		redirectUri
	});

	const scope = ['https://www.googleapis.com/auth/calendar.readonly'];
	const authUrl = oauth2Client.generateAuthUrl({
		access_type: 'offline',
		prompt: 'consent',
		scope
	});

	return new Response(null, {
		status: 302,
		headers: { Location: authUrl }
	});
};
