<script lang="ts">
	import { onMount } from 'svelte';
	import { fade } from 'svelte/transition';
	import type { Article } from './types';
	import dayjs from 'dayjs';
	import { TIMING_STRATEGIES } from '$lib/core/timing';

	let articles: Article[] = [];
	let currentArticle: Article | null = null;
	let loading = true;
	let error: string | null = null;
	let currentIndex = 0;

	let timer: ReturnType<typeof setInterval> | undefined;
	let feedTimer: ReturnType<typeof setInterval> | undefined;

	async function loadArticles() {
		try {
			loading = true;
			error = null;
			const res = await fetch('/api/components/rss-feed');
			if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);

			const data = await res.json();

			if (data.error) {
				error = data.error;
				return;
			}

			articles = data.articles ?? [];
			if (articles.length > 0) {
				currentIndex = 0;
				currentArticle = articles[0];
			} else {
				currentArticle = null;
			}
		} catch (err) {
			error = 'Failed to load RSS feed';
			console.error('Error loading RSS feed:', err);
		} finally {
			loading = false;
		}
	}

	function formatDate(date: string | null) {
		return date ? dayjs(date).format('HH:mm DD/MM') : '';
	}

	onMount(() => {
		loadArticles();

		// Rotate articles every 10 seconds
		timer = setInterval(() => {
			if (articles.length > 0) {
				currentIndex = (currentIndex + 1) % articles.length;
				currentArticle = articles[currentIndex];
			}
		}, TIMING_STRATEGIES.UI.FADE);

		// Refresh feed every 30 minutes
		feedTimer = setInterval(loadArticles, TIMING_STRATEGIES.STANDARD.interval);

		return () => {
			if (timer) clearInterval(timer);
			if (feedTimer) clearInterval(feedTimer);
		};
	});
</script>

<div class="flex flex-col items-center gap-4 select-none">
	{#if loading}
		<div class="flex items-center gap-2 text-2xl opacity-80">
			<span>Loading feed...</span>
		</div>
	{:else if error}
		<div class="flex items-center gap-2 text-2xl opacity-80">
			<span>Feed unavailable</span>
		</div>
	{:else if currentArticle}
		<div class="inline-grid place-items-center">
			{#key currentArticle.title}
				<div class="flex flex-col items-center gap-1 text-center [grid-area:1/1]" transition:fade>
					<span class="text-2xl font-medium">
						{currentArticle.title}
					</span>
					<span class="text-base opacity-80">
						{currentArticle.source}
						{#if currentArticle.date}
							· {formatDate(currentArticle.date)}
						{/if}
					</span>
				</div>
			{/key}
		</div>
	{:else}
		<div class="flex items-center gap-2 text-xl opacity-80">
			<span>No articles available</span>
		</div>
	{/if}
</div>
