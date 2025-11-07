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
	let trackStyle: 'vinyl' | 'square' | 'cassette' = 'vinyl';
	let episodeStyle: 'vinyl' | 'square' | 'cassette' = 'cassette';

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
			? TIMING_STRATEGIES.VERY_FAST.interval // 5 seconds when playing
			: TIMING_STRATEGIES.MEDIUM_FAST.interval; // 30 seconds when paused

		refreshInterval = setInterval(load, interval);
	}

	async function loadConfig() {
		try {
			const res = await fetch('/api/config');
			if (res.ok) {
				const config = await res.json();
				// Support legacy albumArtStyle for backward compatibility
				const legacyStyle = config.components?.spotify?.albumArtStyle;
				trackStyle = config.components?.spotify?.trackStyle || legacyStyle || 'vinyl';
				episodeStyle = config.components?.spotify?.episodeStyle || 'cassette';
			}
		} catch (err) {
			console.error('Error loading config:', err);
			// Default to vinyl for tracks, cassette for episodes
			trackStyle = 'vinyl';
			episodeStyle = 'cassette';
		}
	}

	function getAlbumArtStyle(track: SpotifyTrack): 'vinyl' | 'square' | 'cassette' {
		return track.isEpisode ? episodeStyle : trackStyle;
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
						{@const style = getAlbumArtStyle(track)}
						{#if style === 'vinyl'}
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
						{:else if style === 'cassette'}
							<div class="cassette-container">
								<div class="cassette-body">
									<div class="cassette-label">
										<img class="cassette-label-image" src={track.albumArt} alt="Album art" />
										<div class="cassette-label-overlay"></div>
									</div>
									<div class="cassette-tape-area">
										<div class="cassette-tape-window"></div>
										<div class="cassette-spool {track.isPlaying ? 'cassette-spool-spinning' : ''}">
											<div class="cassette-spool-outer-ring"></div>
											<div class="cassette-spool-center"></div>
											<div class="cassette-spool-gear"></div>
										</div>
										<div class="cassette-spool {track.isPlaying ? 'cassette-spool-spinning' : ''}">
											<div class="cassette-spool-outer-ring"></div>
											<div class="cassette-spool-center"></div>
											<div class="cassette-spool-gear"></div>
										</div>
									</div>
									<div class="cassette-bottom-section"></div>
									<div class="cassette-screw cassette-screw-top-left"></div>
									<div class="cassette-screw cassette-screw-top-right"></div>
									<div class="cassette-screw cassette-screw-bottom-left"></div>
									<div class="cassette-screw cassette-screw-bottom-right"></div>
								</div>
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

	.cassette-container {
		position: relative;
		width: 140px;
		height: 100px;
		flex-shrink: 0;
	}

	.cassette-body {
		position: relative;
		width: 100%;
		height: 100%;
		background: linear-gradient(
			to bottom,
			rgba(35, 35, 35, 1) 0%,
			rgba(28, 28, 28, 1) 50%,
			rgba(22, 22, 22, 1) 100%
		);
		border-radius: 6px;
		box-shadow:
			0 4px 12px rgba(0, 0, 0, 0.4),
			inset 0 1px 0 rgba(255, 255, 255, 0.05);
		overflow: visible;
		border: 1px solid rgba(0, 0, 0, 0.3);
	}

	.cassette-label {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		overflow: hidden;
		z-index: 1;
		border-radius: 6px;
	}

	.cassette-label-image {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.cassette-label-overlay {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background: linear-gradient(
			to bottom,
			rgba(0, 0, 0, 0) 0%,
			rgba(0, 0, 0, 0.1) 70%,
			rgba(0, 0, 0, 0.3) 100%
		);
		pointer-events: none;
	}

	.cassette-tape-area {
		position: absolute;
		top: 20%;
		left: 0;
		width: 100%;
		height: 50%;
		z-index: 3;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 18px;
		box-sizing: border-box;
	}

	.cassette-tape-window {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		width: 60%;
		height: 10px;
		background: rgb(18, 18, 18);
		border: 1px solid rgba(0, 0, 0, 0.6);
		border-radius: 3px;
		box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.5);
		z-index: 1;
	}

	.cassette-spool {
		position: relative;
		width: 28px;
		height: 28px;
		z-index: 4;
		flex-shrink: 0;
		transform-origin: center center;
	}

	.cassette-spool-outer-ring {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		width: 28px;
		height: 28px;
		border-radius: 50%;
		background: rgba(50, 50, 50, 0.9);
		border: 1px solid rgba(30, 30, 30, 0.8);
		box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.6);
		transform-origin: center center;
	}

	.cassette-spool-center {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		width: 20px;
		height: 20px;
		border-radius: 50%;
		background: #ffffff;
		box-shadow:
			inset 0 1px 3px rgba(0, 0, 0, 0.2),
			0 0 1px rgba(0, 0, 0, 0.3);
		z-index: 2;
		transform-origin: center center;
	}

	.cassette-spool-gear {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		width: 28px;
		height: 28px;
		border-radius: 50%;
		background: rgb(75, 75, 75);
		background-image:
			repeating-conic-gradient(from 0deg, rgb(75, 75, 75) 0deg 15deg, rgb(45, 45, 45) 15deg 30deg),
			radial-gradient(
				circle,
				rgb(45, 45, 45) 0%,
				rgb(45, 45, 45) 40%,
				rgb(60, 60, 60) 45%,
				rgb(75, 75, 75) 100%
			);
		border: 1.5px solid rgb(35, 35, 35);
		box-shadow:
			inset 0 0 3px rgba(0, 0, 0, 0.6),
			0 0 1px rgba(0, 0, 0, 0.8);
		z-index: 1;
		transform-origin: center center;
	}

	.cassette-spool-spinning {
		animation: spin 2s linear infinite;
	}

	.cassette-bottom-section {
		position: absolute;
		bottom: 0;
		left: 0;
		width: 100%;
		height: 35%;
		background: rgb(18, 18, 18);
		border-top: 1px solid rgba(0, 0, 0, 0.4);
		z-index: 1;
		border-radius: 0 0 6px 6px;
	}

	.cassette-screw {
		position: absolute;
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: radial-gradient(
			circle,
			rgba(110, 110, 110, 0.9) 0%,
			rgba(80, 80, 80, 0.95) 40%,
			rgba(60, 60, 60, 1) 100%
		);
		box-shadow:
			inset 0 1px 1px rgba(255, 255, 255, 0.15),
			inset 0 -1px 1px rgba(0, 0, 0, 0.7),
			0 0 1px rgba(0, 0, 0, 0.5);
		z-index: 5;
	}

	.cassette-screw::before {
		content: '+';
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		color: rgba(0, 0, 0, 0.7);
		font-size: 6px;
		font-weight: 900;
		line-height: 1;
		font-family: Arial, sans-serif;
	}

	.cassette-screw-top-left {
		top: 6px;
		left: 6px;
	}

	.cassette-screw-top-right {
		top: 6px;
		right: 6px;
	}

	.cassette-screw-bottom-left {
		bottom: 6px;
		left: 6px;
	}

	.cassette-screw-bottom-right {
		bottom: 6px;
		right: 6px;
	}
</style>
