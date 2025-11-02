import { XMLParser } from 'fast-xml-parser';
import type { Article, FeedConfig } from './types';
import { TIMING_STRATEGIES } from '$lib/core/timing';
import { getCached, setCache } from '$lib/core/utils';
import type { CacheEntry } from '$lib/core/utils';

let cache: CacheEntry<Article[]> | null = null;

interface RSSConfig {
	rssFeeds?: FeedConfig[];
}

function decodeNumericEntities(str: string): string {
	if (!str) return '';
	const entityMap: Record<string, string> = {
		'8211': '–',
		'8216': '\u2018', // left single quote
		'8217': '\u2019', // right single quote
		'8230': '…',
		'8220': '\u201C', // left double quote
		'8221': '\u201D', // right double quote
		'8226': '•',
		'38': '&',
		'60': '<',
		'62': '>',
		'34': '"',
		'39': "'",
		'8212': '—',
		'163': '£',
		'169': '©',
		'174': '®',
		'8482': '™',
		'183': '·'
	};
	return str
		.replace(/&#(\d+);/g, (match, dec) => entityMap[dec] ?? match)
		.replace(/&amp;/g, '&')
		.replace(/&lt;/g, '<')
		.replace(/&gt;/g, '>')
		.replace(/&quot;/g, '"')
		.replace(/&apos;/g, "'")
		.replace(/&nbsp;/g, ' ')
		.replace(/&copy;/g, '©')
		.replace(/&reg;/g, '®')
		.replace(/&pound;/g, '£')
		.replace(/&hellip;/g, '…');
}

interface ParsedRSSItem {
	title?: string;
	pubDate?: string;
}

async function fetchFeed(feed: FeedConfig): Promise<Article[]> {
	const res = await fetch(feed.url);
	if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
	const xmlText = await res.text();

	const parser = new XMLParser({
		ignoreAttributes: false,
		attributeNamePrefix: ''
	});
	const data = parser.parse(xmlText);

	const channel = data.rss?.channel || data.feed;
	const feedTitle = feed.sourceName || channel?.title || 'RSS Feed';

	const items = Array.isArray(channel?.item) ? channel.item : channel?.item ? [channel.item] : [];

	return items.map((item: ParsedRSSItem) => ({
		title: item.title ? decodeNumericEntities(item.title) : 'Untitled',
		source: feedTitle,
		date: item.pubDate ? new Date(item.pubDate).toISOString() : ''
	}));
}

export async function GET(config: RSSConfig): Promise<{ articles: Article[] } | { error: string }> {
	try {
		// Check cache
		const cached = getCached(cache);
		if (cached) {
			return { articles: cached };
		}

		const feeds = config.rssFeeds || [];

		if (feeds.length === 0) {
			return { articles: [] };
		}

		const allArticles = await Promise.all(feeds.map((feed) => fetchFeed(feed)));
		const articles = allArticles.flat();

		// Filter to only include articles from the past week
		const oneWeekAgo = new Date();
		oneWeekAgo.setDate(oneWeekAgo.getDate() - 7);

		const recentArticles = articles.filter((article) => {
			if (!article.date) return false;
			const articleDate = new Date(article.date);
			return articleDate >= oneWeekAgo;
		});

		// Sort newest → oldest
		recentArticles.sort((a, b) => {
			const dateA = a.date ? new Date(a.date).getTime() : 0;
			const dateB = b.date ? new Date(b.date).getTime() : 0;
			return dateB - dateA;
		});

		cache = setCache(cache, recentArticles, Date.now() + TIMING_STRATEGIES.STANDARD.interval);

		return { articles: recentArticles };
	} catch (error) {
		console.error('RSS API error:', error);
		return { error: error instanceof Error ? error.message : 'Failed to fetch RSS feeds' };
	}
}
