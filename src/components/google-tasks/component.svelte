<script lang="ts">
	import { onMount } from 'svelte';
	import ListTodo from '@lucide/svelte/icons/list-todo';
	import CircleAlert from '@lucide/svelte/icons/circle-alert';
	import CheckSquare from '@lucide/svelte/icons/check-square';
	import Clock from '@lucide/svelte/icons/clock';
	import { TIMING_STRATEGIES } from '$lib/core/timing';
	import type { GoogleTask } from './types';
	import dayjs from 'dayjs';

	let tasks: GoogleTask[] | null = null;
	let loading = true;
	let error: string | null = null;
	let firstLoad = true;

	/** Returns formatted time, or empty string if date-only (API always returns midnight for date-only tasks) */
	function formatDueTime(due?: string): string {
		if (!due) return '';
		const d = dayjs(due);
		if (!d.isValid()) return '';
		// API doesn't store time - always returns midnight; don't show misleading "All day" or "00:00"
		if (d.hour() === 0 && d.minute() === 0) return '';
		return d.format('H:mm');
	}

	async function loadTasks() {
		try {
			loading = true;
			error = null;

			const res = await fetch('/api/components/google-tasks');
			if (res.status === 401) {
				tasks = null;
				return;
			}
			if (!res.ok) throw new Error(`HTTP ${res.status}`);

			const result = await res.json();

			if (result.error) {
				if (result.error === 'Not authenticated') {
					tasks = null;
					return;
				}
				throw new Error(result.error);
			}

			tasks = result;
		} catch (e) {
			error = 'Failed to load tasks';
			console.error('Failed to fetch Google Tasks:', e);
		} finally {
			loading = false;
			firstLoad = false;
		}
	}

	function connectGoogle() {
		window.location.href = '/api/google/auth';
	}

	onMount(() => {
		loadTasks();
		const interval = setInterval(loadTasks, TIMING_STRATEGIES.FREQUENT.interval);
		return () => clearInterval(interval);
	});
</script>

<div class="flex flex-col items-start gap-3 select-none">
	<div class="flex items-center gap-3 opacity-90">
		<ListTodo size={24} />
		<h2 class="text-lg tracking-wide">Tasks Today</h2>
	</div>
	{#if firstLoad && loading}
		<p class="text-lg">Loading tasks…</p>
	{:else if error}
		<div class="flex items-center gap-2 text-lg text-red-400 opacity-80">
			<CircleAlert size={22} />
			<span>{error}</span>
		</div>
	{:else if tasks === null}
		<div class="flex flex-col gap-2">
			<p class="text-base opacity-80">
				Connect or reconnect Google to show tasks. If you already use Calendar, click to grant
				Tasks access.
			</p>
			<button class="rounded bg-green-600 px-3 py-2 text-white" on:click={connectGoogle}>
				Connect Google Tasks
			</button>
		</div>
	{:else if tasks.length === 0}
		<p class="text-lg opacity-70">No tasks due today</p>
	{:else}
		<div class="flex flex-col gap-2">
			{#each tasks as task (task.id)}
				<div class="flex items-center gap-3">
					<CheckSquare size={20} class="flex-shrink-0 opacity-70" />
					<div class="flex-1 min-w-0">
						<div class="text-lg truncate">{task.title}</div>
						{#if task.due && formatDueTime(task.due)}
							<div class="flex items-center gap-2 text-base opacity-70">
								<Clock size={14} class="flex-shrink-0" />
								{formatDueTime(task.due)}
							</div>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
