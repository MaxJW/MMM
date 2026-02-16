import { google } from 'googleapis';
import dayjs from 'dayjs';
import { TIMING_STRATEGIES } from '$lib/core/timing';
import { TokenStorage } from '$lib/services/tokenStorage';
import { getTasksConfig } from '$lib/config/userConfig';
import type { GoogleTask } from './types';

interface TasksConfig {
	clientId?: string;
	clientSecret?: string;
	maxTasks?: number;
}

class GoogleTasksService {
	private static cache: {
		data: GoogleTask[];
		expiry: number;
		maxTasks: number;
	} | null = null;

	static async getTasksToday(
		origin: string,
		config: TasksConfig
	): Promise<GoogleTask[]> {
		const tokenData = await TokenStorage.loadTokens();
		if (!tokenData) {
			throw new Error('Not authenticated');
		}

		if (!config.clientId || !config.clientSecret) {
			throw new Error('Google OAuth not configured');
		}

		const maxTasks = typeof config.maxTasks === 'number' ? config.maxTasks : 20;

		if (
			this.cache &&
			Date.now() < this.cache.expiry &&
			this.cache.maxTasks === maxTasks
		) {
			return this.cache.data;
		}

		const oauth2Client = new google.auth.OAuth2({
			clientId: config.clientId,
			clientSecret: config.clientSecret,
			redirectUri: `${origin}/api/google/callback`
		});

		oauth2Client.setCredentials({
			refresh_token: tokenData.refreshToken,
			access_token: tokenData.accessToken,
			expiry_date: tokenData.expiryDate
		});

		try {
			const { token } = await oauth2Client.getAccessToken();

			if (token !== tokenData.accessToken) {
				await TokenStorage.saveTokens({
					...tokenData,
					accessToken: token || undefined,
					expiryDate: oauth2Client.credentials.expiry_date || undefined
				});
			}
		} catch (err) {
			console.error('Failed to refresh Google access token', err);
			throw new Error('Authentication failed');
		}

		const tasksApi = google.tasks({ version: 'v1', auth: oauth2Client });

		const dueMin = dayjs().startOf('day').toISOString();
		const dueMax = dayjs().endOf('day').toISOString();

		const res = await tasksApi.tasks.list({
			tasklist: '@default',
			dueMin,
			dueMax,
			showCompleted: false,
			maxResults: maxTasks
		});

		const items = (res.data.items || []) as Array<{
			id?: string;
			title?: string;
			due?: string;
			status?: string;
		}>;

		const tasks: GoogleTask[] = items.map((t) => ({
			id: t.id || '',
			title: t.title || '(No title)',
			due: t.due,
			completed: t.status === 'completed'
		}));

		this.cache = {
			data: tasks,
			expiry: Date.now() + TIMING_STRATEGIES.FREQUENT.interval,
			maxTasks
		};

		return tasks;
	}
}

export async function GET(
	config: TasksConfig,
	request?: Request
): Promise<GoogleTask[] | { error: string }> {
	try {
		const resolvedConfig = config.clientId
			? config
			: await getTasksConfig();

		const origin = request ? new URL(request.url).origin : 'http://localhost:5173';
		const data = await GoogleTasksService.getTasksToday(origin, resolvedConfig);
		return data;
	} catch (error) {
		const err = error as Error & {
			status?: number;
			code?: number;
			response?: { status?: number };
		};
		if (err.message === 'Not authenticated') {
			return { error: 'Not authenticated' };
		}
		// 403 insufficient_scope: user's tokens lack tasks.readonly (need to re-auth)
		const status = err.status ?? err.code ?? err.response?.status;
		const isInsufficientScope =
			status === 403 &&
			(err.message?.toLowerCase().includes('insufficient') ||
				err.message?.toLowerCase().includes('scope'));
		if (isInsufficientScope) {
			return { error: 'Not authenticated' };
		}
		console.error('Failed to fetch Google Tasks', error);
		return { error: 'Failed to fetch tasks' };
	}
}
