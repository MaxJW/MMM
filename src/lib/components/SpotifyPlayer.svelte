<script lang="ts">
	import Music from '@lucide/svelte/icons/music';
	import { onMount } from 'svelte';
	import { SpotifyService } from '$lib/services/spotifyService';
	import type { SpotifyTrack } from '$lib/types/spotify';
	import { TIMING_STRATEGIES } from '$lib/types/util';

	let track: SpotifyTrack | null = null;
	let loading = true;

	async function load() {
		track = await SpotifyService.getCurrentTrack();
		loading = false;
	}

	onMount(() => {
		load();
		const interval = setInterval(load, TIMING_STRATEGIES.FREQUENT.interval);
		return () => clearInterval(interval);
	});
</script>

<div class="flex flex-col items-start gap-3 select-none">
	<div class="flex items-center gap-3 opacity-90">
		<Music size={24} />
		<h2 class="text-lg">Spotify</h2>
	</div>
	{#if loading}
		<p>Loading…</p>
	{:else if track && track.isPlaying}
		<div class="flex items-center gap-3">
			<img class="h-16 w-16 rounded" src={track.albumArt} alt="Album art" />
			<div class="flex flex-col">
				<span class="text-lg font-medium">{track.title}</span>
				<span class="text-base opacity-80">{track.artist}</span>
			</div>
		</div>
	{:else}
		<p>Nothing playing</p>
	{/if}
</div>
