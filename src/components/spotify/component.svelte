<script lang="ts">
	import Music from '@lucide/svelte/icons/music';
	import Smartphone from '@lucide/svelte/icons/smartphone';
	import Speaker from '@lucide/svelte/icons/speaker';
	import User from '@lucide/svelte/icons/user';
	import { onMount, onDestroy } from 'svelte';
	import type { SpotifyTrack } from './types';
	import { TIMING_STRATEGIES } from '$lib/core/timing';

	let tracks: SpotifyTrack[] = [];
	let loading = true;
	let error: string | null = null;
	let refreshInterval: ReturnType<typeof setInterval> | null = null;
	let albumArtStyle: 'vinyl' | 'square' = 'vinyl';

	async function load() {
		try {
			const res = await fetch('/api/components/spotify');
			if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);

			const data = await res.json();

			if (data.error) {
				error = data.error;
				tracks = [];
				setupRefreshInterval(false);
				return;
			}

			// Support both array response (new) and single track response (legacy)
			const newTracks = Array.isArray(data) ? data : data.isPlaying ? [data] : [];

			const wasPlaying = tracks.some((t) => t.isPlaying);
			const isNowPlaying = newTracks.some((t) => t.isPlaying);

			tracks = newTracks;

			// Update refresh interval if playing state changed
			if (wasPlaying !== isNowPlaying) {
				setupRefreshInterval(isNowPlaying);
			}
		} catch (err) {
			error = 'Failed to load Spotify tracks';
			console.error('Error loading Spotify tracks:', err);
			tracks = [];
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
			: TIMING_STRATEGIES.MEDIUM.interval; // 1 minute when paused

		refreshInterval = setInterval(load, interval);
	}

	async function loadConfig() {
		try {
			const res = await fetch('/api/config');
			if (res.ok) {
				const config = await res.json();
				albumArtStyle = config.components?.spotify?.albumArtStyle || 'vinyl';
			}
		} catch (err) {
			console.error('Error loading config:', err);
			// Default to vinyl if config fails to load
			albumArtStyle = 'vinyl';
		}
	}

	onMount(() => {
		loadConfig();
		load();

		// Initial refresh interval will be set after first load based on track state
		// Setup with a default (will update after first load)
		setupRefreshInterval(false);

		return () => {
			if (refreshInterval) clearInterval(refreshInterval);
		};
	});

	onDestroy(() => {
		if (refreshInterval) clearInterval(refreshInterval);
	});
</script>

{#if tracks.length > 0}
	<div class="flex flex-col items-start gap-3 select-none">
		<div class="flex items-center gap-3 opacity-90">
			<Music size={24} />
			<h2 class="text-lg">Spotify</h2>
		</div>

		<div class="flex flex-col gap-4">
			{#each tracks as track}
				<div class="flex items-center gap-4">
					{#if track.albumArt}
						{#if albumArtStyle === 'vinyl'}
							<div class="vinyl-container">
								<img
									class="vinyl-image {track.isPlaying ? 'vinyl-spinning' : ''}"
									src={track.albumArt}
									alt="Album art"
								/>
								<div class="vinyl-overlay {track.isPlaying ? 'vinyl-spinning' : ''}"></div>
								<div class="vinyl-gloss"></div>
								<div class="vinyl-center-background"></div>
								<div class="vinyl-center"></div>
								<div class="vinyl-center-hole"></div>
							</div>
						{:else}
							<div class="album-art-square">
								<img class="album-art-image" src={track.albumArt} alt="Album art" />
							</div>
						{/if}
					{/if}
					<div class="flex min-w-0 flex-col">
						<span class="truncate text-xl font-medium">{track.title}</span>
						<span class="truncate text-base opacity-80">{track.artist}</span>
						{#if track.accountName}
							<div class="mt-1 flex items-center gap-1.5">
								<User size={14} class="flex-shrink-0 opacity-70" />
								<span class="truncate text-sm opacity-70">{track.accountName}</span>
							</div>
						{/if}
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
			{/each}
		</div>
	</div>
{/if}

<style>
	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}

	.vinyl-container {
		position: relative;
		width: 128px;
		height: 128px;
		flex-shrink: 0;
	}

	.vinyl-image {
		width: 100%;
		height: 100%;
		border-radius: 50%;
		object-fit: cover;
		box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
	}

	.vinyl-overlay {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		border-radius: 50%;
		background: repeating-radial-gradient(
			transparent 0,
			transparent 18%,
			rgba(0, 0, 0, 0.04) 18.5%,
			rgba(0, 0, 0, 0.04) 19%,
			transparent 19%,
			transparent 24%,
			rgba(0, 0, 0, 0.05) 24.5%,
			transparent 25%,
			transparent 29%,
			rgba(0, 0, 0, 0.06) 29.5%,
			rgba(0, 0, 0, 0.06) 30%,
			transparent 30%,
			transparent 34%,
			rgba(0, 0, 0, 0.07) 34.5%,
			rgba(0, 0, 0, 0.07) 35%,
			transparent 35%,
			transparent 39%,
			rgba(0, 0, 0, 0.08) 39.5%,
			rgba(0, 0, 0, 0.08) 40%,
			transparent 40%,
			transparent 44%,
			rgba(0, 0, 0, 0.09) 44.5%,
			rgba(0, 0, 0, 0.09) 45%,
			transparent 45%,
			transparent 49%,
			rgba(0, 0, 0, 0.1) 49.5%,
			rgba(0, 0, 0, 0.1) 50%,
			transparent 50%,
			transparent 54%,
			rgba(0, 0, 0, 0.11) 54.5%,
			rgba(0, 0, 0, 0.11) 55%,
			transparent 55%,
			transparent 60%,
			rgba(0, 0, 0, 0.12) 60.5%,
			rgba(0, 0, 0, 0.12) 61%,
			transparent 61%,
			transparent 65%,
			rgba(0, 0, 0, 0.13) 65.5%,
			rgba(0, 0, 0, 0.13) 66%,
			transparent 66%,
			transparent 70%,
			rgba(0, 0, 0, 0.14) 70.5%,
			rgba(0, 0, 0, 0.14) 71%,
			transparent 71%,
			transparent 75%,
			rgba(0, 0, 0, 0.15) 75.5%,
			rgba(0, 0, 0, 0.15) 76%,
			transparent 76%,
			rgba(0, 0, 0, 0.18) 85%,
			rgba(0, 0, 0, 0.2) 100%
		);
		pointer-events: none;
		z-index: 1;
	}

	.vinyl-center-background {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		width: 22px;
		height: 22px;
		border-radius: 50%;
		background: radial-gradient(
			circle,
			rgba(40, 40, 40, 0.8) 0%,
			rgba(20, 20, 20, 0.9) 70%,
			rgba(10, 10, 10, 1) 100%
		);
		box-shadow:
			inset 0 1px 2px rgba(255, 255, 255, 0.1),
			inset 0 -1px 2px rgba(0, 0, 0, 0.8),
			0 0 4px rgba(0, 0, 0, 0.6);
		pointer-events: none;
		z-index: 3;
	}

	.vinyl-center {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background: radial-gradient(
			circle,
			rgba(30, 30, 30, 0.9) 0%,
			rgba(15, 15, 15, 1) 50%,
			rgba(5, 5, 5, 1) 100%
		);
		box-shadow:
			inset 0 1px 3px rgba(255, 255, 255, 0.15),
			inset 0 -1px 4px rgba(0, 0, 0, 0.9),
			0 0 2px rgba(0, 0, 0, 0.8);
		pointer-events: none;
		z-index: 4;
	}

	.vinyl-center-hole {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: rgba(0, 0, 0, 0.95);
		box-shadow:
			inset 0 0 3px rgba(0, 0, 0, 1),
			0 0 1px rgba(255, 255, 255, 0.05);
		pointer-events: none;
		z-index: 5;
	}

	.vinyl-spinning {
		animation: spin 14s linear infinite;
	}

	.album-art-square {
		position: relative;
		width: 128px;
		height: 128px;
		flex-shrink: 0;
	}

	.album-art-image {
		width: 100%;
		height: 100%;
		border-radius: 12px;
		object-fit: cover;
		box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
	}
</style>
