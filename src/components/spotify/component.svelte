<script lang="ts">
	import Music from '@lucide/svelte/icons/music';
	import Smartphone from '@lucide/svelte/icons/smartphone';
	import Speaker from '@lucide/svelte/icons/speaker';
	import { onMount, onDestroy } from 'svelte';
	import type { SpotifyTrack } from './types';
	import { TIMING_STRATEGIES } from '$lib/core/timing';

	let track: SpotifyTrack | null = null;
	let loading = true;
	let error: string | null = null;
	let progressMs = 0;
	let progressUpdateInterval: ReturnType<typeof setInterval> | null = null;
	let refreshInterval: ReturnType<typeof setInterval> | null = null;
	let lastUpdateTime = Date.now();

	function formatTime(ms: number): string {
		const seconds = Math.floor(ms / 1000);
		const minutes = Math.floor(seconds / 60);
		const remainingSeconds = seconds % 60;
		return `${minutes}:${remainingSeconds.toString().padStart(2, '0')}`;
	}

	function getProgressPercentage(): number {
		if (!track || track.durationMs === 0) return 0;
		return Math.min((progressMs / track.durationMs) * 100, 100);
	}

	async function load() {
		try {
			const res = await fetch('/api/components/spotify');
			if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);

			const data = await res.json();

			if (data.error) {
				error = data.error;
				track = null;
				progressMs = 0;
				setupRefreshInterval(false);
				return;
			}

			const wasPlaying = track?.isPlaying ?? false;
			track = data;
			progressMs = data.progressMs || 0;
			lastUpdateTime = Date.now();

			// Update refresh interval if playing state changed
			if (wasPlaying !== data.isPlaying) {
				setupRefreshInterval(data.isPlaying);
			}
		} catch (err) {
			error = 'Failed to load Spotify track';
			console.error('Error loading Spotify track:', err);
			track = null;
			progressMs = 0;
			setupRefreshInterval(false);
		} finally {
			loading = false;
		}
	}

	function setupRefreshInterval(isPlaying: boolean) {
		// Clear existing interval
		if (refreshInterval) {
			clearInterval(refreshInterval);
			refreshInterval = null;
		}

		// Set new interval based on playing state
		const interval = isPlaying
			? TIMING_STRATEGIES.FAST.interval // 10 seconds when playing
			: TIMING_STRATEGIES.FAST.interval; // 1 minute when paused

		refreshInterval = setInterval(load, interval);
	}

	function updateProgress() {
		if (track && track.isPlaying && track.durationMs > 0) {
			const now = Date.now();
			const elapsed = now - lastUpdateTime;
			progressMs = Math.min(progressMs + elapsed, track.durationMs);
			lastUpdateTime = now;
		}
	}

	onMount(() => {
		load();

		// Initial refresh interval will be set after first load based on track state
		// Setup with a default (will update after first load)
		setupRefreshInterval(false);

		// Update progress bar in real-time when playing
		progressUpdateInterval = setInterval(() => {
			updateProgress();
		}, TIMING_STRATEGIES.REALTIME.interval);

		return () => {
			if (refreshInterval) clearInterval(refreshInterval);
			if (progressUpdateInterval) clearInterval(progressUpdateInterval);
		};
	});

	onDestroy(() => {
		if (refreshInterval) clearInterval(refreshInterval);
		if (progressUpdateInterval) clearInterval(progressUpdateInterval);
	});
</script>

{#if track && track.title && track.isPlaying}
	<div class="flex flex-col items-start gap-3 select-none">
		<div class="flex items-center gap-3 opacity-90">
			<Music size={24} />
			<h2 class="text-lg">Spotify</h2>
		</div>

		<div class="flex flex-col gap-3">
			<div class="flex items-center gap-4">
				{#if track.albumArt}
					<img
						class="h-20 w-20 flex-shrink-0 rounded-lg shadow-lg"
						src={track.albumArt}
						alt="Album art"
					/>
				{/if}
				<div class="flex min-w-0 flex-col">
					<span class="truncate text-xl font-medium">{track.title}</span>
					<span class="truncate text-base opacity-80">{track.artist}</span>
					{#if track.deviceName && track.deviceName !== 'Unknown Device'}
						<div class="mt-1 flex items-center gap-1.5">
							{#if track.deviceName.toLowerCase().includes('iphone')}
								<Smartphone size={14} class="flex-shrink-0 opacity-70" />
							{:else}
								<Speaker size={14} class="flex-shrink-0 opacity-70" />
							{/if}
							<span class="truncate text-sm opacity-70">Playing on {track.deviceName}</span>
						</div>
					{/if}
				</div>
			</div>

			{#if track.durationMs > 0}
				<div class="flex flex-col gap-1">
					<!-- Progress bar -->
					<div class="relative h-1 overflow-hidden rounded-full bg-neutral-800">
						<div
							class="h-full rounded-full bg-neutral-100 transition-all duration-1000 ease-linear"
							style="width: {getProgressPercentage()}%"
						></div>
					</div>
					<!-- Time display -->
					<div class="flex justify-between text-sm tabular-nums opacity-70">
						<span>{formatTime(progressMs)}</span>
						<span>{formatTime(track.durationMs)}</span>
					</div>
				</div>
			{/if}
		</div>
	</div>
{/if}
