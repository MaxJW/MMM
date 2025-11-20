import type { PolestarConfig, PolestarData } from './types';
import { TIMING_STRATEGIES } from '$lib/core/timing';
import { getCached, setCache } from '$lib/core/utils';
import type { CacheEntry } from '$lib/core/utils';
import crypto from 'node:crypto';
import { Buffer } from 'node:buffer';

// Module-level caches
let dataCache: CacheEntry<PolestarData> | null = null;
let tokenCache: { accessToken: string; expiresAt: number } | null = null;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let oidcConfigCache: any = null;

const CONSTANTS = {
	OIDC_PROVIDER_BASE_URL: 'https://polestarid.eu.polestar.com',
	OIDC_CLIENT_ID: 'l3oopkc_10',
	OIDC_REDIRECT_URI: 'https://www.polestar.com/sign-in-callback',
	OIDC_SCOPE: 'openid profile email customer:attributes',
	API_MYSTAR_V2_URL: 'https://pc-api.polestar.com/eu-north-1/mystar-v2/',
	USER_AGENT: 'Polestar/4.0.0 (iPhone; iOS 17.0; Scale/3.00)'
};

const QUERIES = {
	TELEMATICS_V2: `
    query CarTelematicsV2($vins: [String!]!) {
        carTelematicsV2(vins: $vins) {
            battery {
                batteryChargeLevelPercentage
                chargingStatus
                estimatedChargingTimeToFullMinutes
                estimatedDistanceToEmptyKm
            }
            odometer {
                odometerMeters
            }
        }
    }
  `
};

// ==========================================
// Utils & Cookie Jar
// ==========================================

class SimpleCookieJar {
	private cookies = new Map<string, string>();

	public update(headers: Headers) {
		let cookies: string[] = [];
		if (typeof headers.getSetCookie === 'function') {
			cookies = headers.getSetCookie();
		} else {
			const header = headers.get('set-cookie');
			if (header) cookies = [header];
		}

		cookies.forEach((cookieStr) => {
			const [nameValue] = cookieStr.split(';');
			if (nameValue) {
				const [name, ...valParts] = nameValue.split('=');
				this.cookies.set(name.trim(), valParts.join('=').trim());
			}
		});
	}

	public getHeader(): string {
		return Array.from(this.cookies)
			.map(([k, v]) => `${k}=${v}`)
			.join('; ');
	}
}

const Utils = {
	base64UrlEncode: (str: Buffer): string => {
		return str.toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
	},

	generateCodeVerifier: (): string => Utils.base64UrlEncode(crypto.randomBytes(32)),

	generateCodeChallenge: (verifier: string): string => {
		const hash = crypto.createHash('sha256').update(verifier).digest();
		return Utils.base64UrlEncode(hash);
	},

	generateState: (): string => Utils.base64UrlEncode(crypto.randomBytes(32))
};

// ==========================================
// Polestar Client
// ==========================================

class PolestarClient {
	private cookieJar = new SimpleCookieJar();
	private accessToken: string | null = null;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	private oidcConfig: any = null;

	private async fetch(url: string, options: RequestInit = {}) {
		const headers = new Headers(options.headers || {});
		const cookieHeader = this.cookieJar.getHeader();
		if (cookieHeader) headers.set('Cookie', cookieHeader);
		if (!headers.has('User-Agent')) headers.set('User-Agent', CONSTANTS.USER_AGENT);

		const response = await fetch(url, { ...options, headers });
		this.cookieJar.update(response.headers);
		return response;
	}

	private async getOidcConfiguration() {
		if (oidcConfigCache) {
			this.oidcConfig = oidcConfigCache;
			return;
		}
		const url = `${CONSTANTS.OIDC_PROVIDER_BASE_URL}/.well-known/openid-configuration`;
		const response = await this.fetch(url);
		if (!response.ok) throw new Error('Failed to fetch OIDC config');
		this.oidcConfig = await response.json();
		oidcConfigCache = this.oidcConfig;
	}

	public async getAccessToken(username: string, pass: string): Promise<string> {
		// Use cached token if valid
		if (tokenCache && tokenCache.expiresAt > Date.now()) {
			this.accessToken = tokenCache.accessToken;
			return this.accessToken;
		}
		return this.login(username, pass);
	}

	private async login(username: string, pass: string): Promise<string> {
		if (!this.oidcConfig) await this.getOidcConfiguration();

		const state = Utils.generateState();
		const codeVerifier = Utils.generateCodeVerifier();
		const codeChallenge = Utils.generateCodeChallenge(codeVerifier);

		// 1. Build Auth URL
		const params = new URLSearchParams({
			client_id: CONSTANTS.OIDC_CLIENT_ID,
			redirect_uri: CONSTANTS.OIDC_REDIRECT_URI,
			response_type: 'code',
			scope: CONSTANTS.OIDC_SCOPE,
			state: state,
			code_challenge: codeChallenge,
			code_challenge_method: 'S256',
			response_mode: 'query'
		});

		// 2. Get Resume Path
		const authUrl = `${this.oidcConfig.authorization_endpoint}?${params.toString()}`;
		const authResp = await this.fetch(authUrl);
		const authText = await authResp.text();

		const resumePathMatch = authText.match(/(?:url|action):\s*"(.+)"/);
		if (!resumePathMatch) throw new Error('Could not extract resume path from login page.');
		const resumePath = resumePathMatch[1];

		// 3. Post Credentials
		const loginUrl = new URL(resumePath, CONSTANTS.OIDC_PROVIDER_BASE_URL).toString();
		const loginData = new URLSearchParams();
		loginData.append('pf.username', username);
		loginData.append('pf.pass', pass);

		let currentResp = await this.fetch(loginUrl, {
			method: 'POST',
			body: loginData,
			redirect: 'manual'
		});

		// 4. Follow Redirects
		let code: string | null = null;
		for (let i = 0; i < 5; i++) {
			if (currentResp.status >= 300 && currentResp.status < 400) {
				await currentResp.text(); // Consume body to free socket
				const location = currentResp.headers.get('Location');
				if (!location) throw new Error('Redirect with no Location header');

				const locationUrl = new URL(location, CONSTANTS.OIDC_PROVIDER_BASE_URL);
				const codeParam = locationUrl.searchParams.get('code');

				if (codeParam) {
					code = codeParam;
					break;
				}

				const uid = locationUrl.searchParams.get('uid');
				if (uid) {
					const submitData = new URLSearchParams();
					submitData.append('pf.submit', 'true');
					submitData.append('subject', uid);
					currentResp = await this.fetch(loginUrl, {
						method: 'POST',
						body: submitData,
						redirect: 'manual'
					});
					continue;
				}

				currentResp = await this.fetch(locationUrl.toString(), {
					method: 'GET',
					redirect: 'manual'
				});
			} else {
				const text = await currentResp.text();
				if (text.includes('authMessage: "ERR001"')) {
					throw new Error('Authentication failed: Invalid credentials');
				}
				throw new Error(`Unexpected status ${currentResp.status} during login flow.`);
			}
		}

		if (!code) throw new Error('Failed to retrieve authorization code.');

		// 5. Exchange Code for Token
		const tokenParams = new URLSearchParams();
		tokenParams.append('grant_type', 'authorization_code');
		tokenParams.append('client_id', CONSTANTS.OIDC_CLIENT_ID);
		tokenParams.append('code', code);
		tokenParams.append('redirect_uri', CONSTANTS.OIDC_REDIRECT_URI);
		tokenParams.append('code_verifier', codeVerifier);

		const tokenResp = await this.fetch(this.oidcConfig.token_endpoint, {
			method: 'POST',
			body: tokenParams
		});

		const tokenData = await tokenResp.json();
		if (tokenData.error) throw new Error(`Token Error: ${tokenData.error}`);

		this.accessToken = tokenData.access_token;

		// Cache token (buffer 60s)
		const expiresIn = tokenData.expires_in || 3600;
		tokenCache = {
			accessToken: this.accessToken!,
			expiresAt: Date.now() + expiresIn * 1000 - 60000
		};

		return this.accessToken!;
	}

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	public async queryGraphql(query: string, variables: any = {}) {
		if (!this.accessToken) throw new Error('Not authenticated.');

		const response = await this.fetch(CONSTANTS.API_MYSTAR_V2_URL, {
			method: 'POST',
			headers: {
				Authorization: `Bearer ${this.accessToken}`,
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({ query, variables })
		});

		const body = await response.json();

		if (body.errors) {
			const errCode = body.errors[0]?.extensions?.code;
			if (errCode === 'UNAUTHENTICATED') {
				throw new Error('Token expired or invalid.');
			}
			throw new Error(`GraphQL Error: ${JSON.stringify(body.errors)}`);
		}

		return body.data;
	}

	public async getTelematics(vin: string) {
		const data = await this.queryGraphql(QUERIES.TELEMATICS_V2, { vins: [vin] });
		return data.carTelematicsV2;
	}
}

export async function GET(config: PolestarConfig): Promise<PolestarData | { error: string }> {
	try {
		if (!config.username || !config.password || !config.vin) {
			throw new Error('Missing Polestar credentials (username, password, vin)');
		}

		const cached = getCached(dataCache);
		if (cached) return cached;

		const client = new PolestarClient();

		// Get valid token (cached or login)
		try {
			await client.getAccessToken(config.username, config.password);
		} catch (e) {
			// If initial login failed, just throw
			console.error('Polestar login failed:', e);
			throw e;
		}

		// Fetch Telematics - with retry for expired tokens
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		let rawData: any;
		try {
			rawData = await client.getTelematics(config.vin);
		} catch (error) {
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			if ((error as any).message === 'Token expired or invalid.') {
				console.log('Polestar token expired, refreshing...');
				tokenCache = null; // Clear cache
				await client.getAccessToken(config.username, config.password); // Force new login
				rawData = await client.getTelematics(config.vin); // Retry fetch
			} else {
				throw error;
			}
		}

		// The API returns an array (likely one entry per VIN requested)
		// We requested just one VIN, so we take the first element
		const carData = Array.isArray(rawData) ? rawData[0] : rawData;

		if (!carData) {
			throw new Error('No telematics data found for this VIN');
		}

		// Simplify the response object structure
		// The battery and odometer fields are also arrays in the response
		const data: PolestarData = {
			battery: Array.isArray(carData.battery) ? carData.battery[0] : carData.battery,
			odometer: Array.isArray(carData.odometer) ? carData.odometer[0] : carData.odometer
		};

		dataCache = setCache(dataCache, data, Date.now() + TIMING_STRATEGIES.INFREQUENT.interval);
		return data;
	} catch (error) {
		console.error('Polestar API error:', error);
		return { error: error instanceof Error ? error.message : 'Failed to fetch Polestar data' };
	}
}
