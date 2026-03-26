import { google } from 'googleapis';
import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';
import timezone from 'dayjs/plugin/timezone';

dayjs.extend(utc);
dayjs.extend(timezone);
import { getAuthenticatedClient } from '$lib/services/googleAuth';
import { getTasksConfig } from '$lib/config/userConfig';
import type { GoogleTask } from './types';

interface TasksConfig {
	clientId?: string;
	clientSecret?: string;
	maxTasks?: number;
	timezone?: string;
	includeTasksWithoutDue?: boolean;
}

async function getTasksToday(
	origin: string,
	config: TasksConfig
): Promise<GoogleTask[]> {
	if (!config.clientId || !config.clientSecret) throw new Error('Google OAuth not configured');

	const maxTasks = typeof config.maxTasks === 'number' ? config.maxTasks : 20;
	const includeUndated =
		config.includeTasksWithoutDue === undefined ||
		config.includeTasksWithoutDue === true ||
		String(config.includeTasksWithoutDue) === 'true';

	const oauth2Client = await getAuthenticatedClient({
		clientId: config.clientId,
		clientSecret: config.clientSecret,
		redirectUri: `${origin}/api/google/callback`
	});

	const tasksApi = google.tasks({ version: 'v1', auth: oauth2Client });
	const now = config.timezone ? dayjs().tz(config.timezone) : dayjs();
	const todayStr = now.format('YYYY-MM-DD');

	const res = await tasksApi.tasks.list({
		tasklist: '@default',
		showCompleted: false,
		maxResults: 100
	});

	type TaskItem = { id?: string; title?: string; due?: string; status?: string };
	const items = (res.data.items || []) as TaskItem[];

	const isDueToday = (due?: string) => {
		if (!due) return false;
		const dueDateStr = due.includes('T') ? due.split('T')[0] : due.substring(0, 10);
		return dueDateStr === todayStr;
	};

	const filtered = items.filter((t) => {
		if (t.due) return isDueToday(t.due);
		return includeUndated;
	});

	filtered.sort((a, b) => {
		if (!a.due && !b.due) return 0;
		if (!a.due) return 1;
		if (!b.due) return -1;
		return new Date(a.due).getTime() - new Date(b.due).getTime();
	});

	return filtered.slice(0, maxTasks).map((t) => ({
		id: t.id || '',
		title: t.title || '(No title)',
		due: t.due,
		completed: t.status === 'completed'
	}));
}

export async function GET(
	config: TasksConfig,
	request?: Request
): Promise<GoogleTask[] | { error: string }> {
	try {
		const resolvedConfig = config.clientId ? config : await getTasksConfig();
		const origin = request ? new URL(request.url).origin : 'http://localhost:5173';
		return await getTasksToday(origin, resolvedConfig);
	} catch (error) {
		const err = error as Error & { status?: number; code?: number; response?: { status?: number } };
		if (err.message === 'Not authenticated') return { error: 'Not authenticated' };
		const status = err.status ?? err.code ?? err.response?.status;
		const isInsufficientScope =
			status === 403 &&
			(err.message?.toLowerCase().includes('insufficient') ||
				err.message?.toLowerCase().includes('scope'));
		if (isInsufficientScope) return { error: 'Not authenticated' };
		console.error('Failed to fetch Google Tasks', error);
		return { error: 'Failed to fetch tasks' };
	}
}
