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

			let token: string | null | undefined;
			try {
				({ token } = await oauth2Client.getAccessToken());
			} catch (err) {
				// A revoked/expired refresh token surfaces as `invalid_grant`. The stored
				// token is dead and will fail forever, so clear it and signal re-auth. This
				// also makes the next auth flow force Google's consent screen (see
				// src/routes/api/google/auth/+server.ts), yielding a fresh refresh token.
				const e = err as Error & { response?: { data?: { error?: string } } };
				const isInvalidGrant =
					e.message?.includes('invalid_grant') || e.response?.data?.error === 'invalid_grant';
				if (isInvalidGrant) {
					await TokenStorage.deleteTokens();
					throw new Error('Not authenticated');
				}
				// Anything else (e.g. transient network errors) is not the token's fault —
				// leave it in place so the component self-heals once connectivity returns.
				throw err;
			}

			const newRefreshToken = oauth2Client.credentials.refresh_token || tokenData.refreshToken;
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
