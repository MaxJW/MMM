import { google } from 'googleapis';
import type { OAuth2Client } from 'google-auth-library';
import { TokenStorage } from './tokenStorage';

interface GoogleAuthConfig {
	clientId: string;
	clientSecret: string;
	redirectUri: string;
}

let refreshPromise: Promise<OAuth2Client> | null = null;

/**
 * Returns an authenticated OAuth2 client with a valid access token.
 * Uses a mutex so concurrent callers (e.g. calendar + tasks) share a
 * single in-flight refresh instead of racing against each other.
 * Also persists any rotated refresh token Google may return.
 */
export async function getAuthenticatedClient(config: GoogleAuthConfig): Promise<OAuth2Client> {
	if (refreshPromise) {
		return refreshPromise;
	}

	refreshPromise = (async () => {
		try {
			const tokenData = await TokenStorage.loadTokens();
			if (!tokenData) throw new Error('Not authenticated');

			const oauth2Client = new google.auth.OAuth2({
				clientId: config.clientId,
				clientSecret: config.clientSecret,
				redirectUri: config.redirectUri
			});

			oauth2Client.setCredentials({
				refresh_token: tokenData.refreshToken,
				access_token: tokenData.accessToken,
				expiry_date: tokenData.expiryDate
			});

			const { token } = await oauth2Client.getAccessToken();

			const newRefreshToken =
				oauth2Client.credentials.refresh_token || tokenData.refreshToken;
			const newExpiry = oauth2Client.credentials.expiry_date || undefined;

			if (
				token !== tokenData.accessToken ||
				newRefreshToken !== tokenData.refreshToken ||
				newExpiry !== tokenData.expiryDate
			) {
				await TokenStorage.saveTokens({
					refreshToken: newRefreshToken,
					accessToken: token || undefined,
					expiryDate: newExpiry
				});
			}

			return oauth2Client;
		} finally {
			refreshPromise = null;
		}
	})();

	return refreshPromise;
}
