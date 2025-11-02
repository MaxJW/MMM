import type { RequestHandler } from './$types';
import { getGoogleConfig } from '$lib/config/userConfig';
import { google } from 'googleapis';

export const GET: RequestHandler = async (event) => {
	const { url } = event;
	const config = await getGoogleConfig();

	if (!config.clientId || !config.clientSecret) {
		return new Response('Google OAuth not configured', { status: 500 });
	}

	const redirectUri = `${url.origin}/api/google/callback`;

	const oauth2Client = new google.auth.OAuth2({
		clientId: config.clientId,
		clientSecret: config.clientSecret,
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
