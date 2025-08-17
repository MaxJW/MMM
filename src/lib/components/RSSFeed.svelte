<script lang="ts">
	import { onMount } from 'svelte';
	import { fade } from 'svelte/transition';
	import { RSSService } from '$lib/services/rssService';
	import type { Article } from '$lib/types/rss';
	import dayjs from 'dayjs';
	import { THIRTY_MIN } from '$lib/types/util';

	let articles: Article[] = [];
	let currentArticle: Article | null = null;
	let loading = true;
	let error: string | null = null;
	let currentIndex = 0;

	let timer: ReturnType<typeof setInterval> | undefined;

	const FADE_INTERVAL = 10000; // 10 seconds per article

	async function loadArticles() {
		try {
			loading = true;
			error = null;
			const result = await RSSService.getArticles();
			articles = result ?? [];
			if (articles.length > 0) {
				currentIndex = 0;
				currentArticle = articles[0];
			} else {
				currentArticle = null;
			}
		} catch (err) {
			error = 'Failed to load RSS feed';
			console.error(err);
		} finally {
			loading = false;
		}
	}

	function formatDate(date: string | null) {
		return date ? dayjs(date).format('HH:mm DD/MM') : '';
	}

	onMount(() => {
		loadArticles();

		// Rotate articles every FADE_INTERVAL milliseconds
		timer = setInterval(() => {
			if (articles.length > 0) {
				currentIndex = (currentIndex + 1) % articles.length;
				currentArticle = articles[currentIndex];
			}
		}, FADE_INTERVAL);

		// Refresh feed every 10 minutes
		const feedInterval = setInterval(loadArticles, THIRTY_MIN);

		return () => {
			if (timer) clearInterval(timer);
			clearInterval(feedInterval);
		};
	});
</script>

<div class="flex flex-col items-center gap-4 select-none">
	{#if loading}
		<div class="flex items-center gap-2 text-xl opacity-70">
			<span>Loading feed...</span>
		</div>
	{:else if error}
		<div class="flex items-center gap-2 text-xl opacity-70">
			<span>Feed unavailable</span>
		</div>
	{:else if currentArticle}
		<div class="inline-grid place-items-center">
			{#key currentArticle.title}
				<!-- put the whole article in one fading block -->
				<div class="flex flex-col items-center gap-1 text-center [grid-area:1/1]" transition:fade>
					<span class="text-xl font-medium">
						{currentArticle.title}
					</span>
					<span class="text-sm opacity-70">
						{currentArticle.source}
						{#if currentArticle.date}
							· {formatDate(currentArticle.date)}
						{/if}
					</span>
				</div>
			{/key}
		</div>
	{:else}
		<div class="flex items-center gap-2 text-xl opacity-70">
			<span>No articles available</span>
		</div>
	{/if}
</div>
