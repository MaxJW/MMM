import type { RequestHandler } from './$types';
import { getComponentConfig, loadConfig, saveConfig } from '$lib/core/config';

export const GET: RequestHandler = async ({ url }) => {
	const code = url.searchParams.get('code');
	const error = url.searchParams.get('error');

	if (error) {
		return new Response(null, {
			status: 302,
			headers: { Location: `${url.origin}/settings?spotify_error=${encodeURIComponent(error)}` }
		});
	}

	if (!code) {
		return new Response(null, {
			status: 302,
			headers: { Location: `${url.origin}/settings?spotify_error=missing_code` }
		});
	}

	const config = await getComponentConfig('spotify');
	if (!config.clientId || !config.clientSecret) {
		return new Response(null, {
			status: 302,
			headers: { Location: `${url.origin}/settings?spotify_error=not_configured` }
		});
	}

	const redirectUri = `${url.origin}/api/spotify/callback`;

	try {
		// Exchange code for tokens
		const tokenResponse = await fetch('https://accounts.spotify.com/api/token', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/x-www-form-urlencoded',
				Authorization: `Basic ${Buffer.from(`${config.clientId}:${config.clientSecret}`).toString('base64')}`
			},
			body: new URLSearchParams({
				grant_type: 'authorization_code',
				code,
				redirect_uri: redirectUri
			})
		});

		if (!tokenResponse.ok) {
			const errorData = await tokenResponse.json().catch(() => ({}));
			console.error('Spotify token exchange failed:', errorData);
			return new Response(null, {
				status: 302,
				headers: {
					Location: `${url.origin}/settings?spotify_error=${encodeURIComponent(errorData.error_description || 'token_exchange_failed')}`
				}
			});
		}

		const tokens = await tokenResponse.json();

		if (!tokens.refresh_token) {
			return new Response(null, {
				status: 302,
				headers: { Location: `${url.origin}/settings?spotify_error=no_refresh_token` }
			});
		}

		// Fetch user profile to get display name
		let userName: string | undefined;
		if (tokens.access_token) {
			try {
				const profileResponse = await fetch('https://api.spotify.com/v1/me', {
					headers: {
						Authorization: `Bearer ${tokens.access_token}`
					}
				});

				if (profileResponse.ok) {
					const profile = await profileResponse.json();
					userName = profile.display_name || profile.id || undefined;
				}
			} catch (err) {
				console.error('Failed to fetch Spotify profile:', err);
				// Continue without user name
			}
		}

		// Load current config and add the new account
		const userConfig = await loadConfig();
		if (!userConfig.components.spotify) {
			userConfig.components.spotify = {};
		}

		const accounts =
			(userConfig.components.spotify.accounts as Array<{ refreshToken: string; name?: string }>) ||
			[];

		// Add new account
		accounts.push({
			refreshToken: tokens.refresh_token,
			name: userName
		});

		userConfig.components.spotify.accounts = accounts;

		// Save updated config
		await saveConfig(userConfig);

		return new Response(null, {
			status: 302,
			headers: { Location: `${url.origin}/settings?connected=spotify` }
		});
	} catch (err) {
		console.error('Spotify OAuth callback failed', err);
		return new Response(null, {
			status: 302,
			headers: { Location: `${url.origin}/settings?spotify_error=callback_failed` }
		});
	}
};
