// src/routes/api/rss/+server.ts
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { XMLParser } from 'fast-xml-parser';
import type { Article } from '$lib/types/rss';
import { TIMING_STRATEGIES } from '$lib/types/util';

interface FeedConfig {
	url: string;
	sourceName?: string;
}

interface ParsedRSSItem {
	title?: string;
	pubDate?: string;
}

class RSSService {
	private static cache: { articles: Article[]; expiry: number } | null = null;

	private static feeds: FeedConfig[] = [
		{
			url: 'https://feeds.skynews.com/feeds/rss/home.xml',
			sourceName: 'Sky News'
		},
		{
			url: 'https://www.thecourier.co.uk/feed/',
			sourceName: 'The Courier'
		}
	];

	private static decodeNumericEntities(str: string): string {
		return str
			.replace(/&#8211;/g, '–') // en dash
			.replace(/&#8216;/g, '‘') // left single quote
			.replace(/&#8217;/g, '’'); // right single quote
	}

	static async fetchFeed(feed: FeedConfig): Promise<Article[]> {
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
			title: item.title ? this.decodeNumericEntities(item.title) : 'Untitled',
			source: feedTitle,
			date: item.pubDate ? new Date(item.pubDate).toISOString() : ''
		}));
	}

	static async getArticles(): Promise<{ articles: Article[] }> {
		if (this.cache && Date.now() < this.cache.expiry) return { articles: this.cache.articles };

		const allArticles = await Promise.all(this.feeds.map((feed) => this.fetchFeed(feed)));
		const articles = allArticles.flat();

		// Sort newest → oldest
		articles.sort((a, b) => {
			const dateA = a.date ? new Date(a.date).getTime() : 0;
			const dateB = b.date ? new Date(b.date).getTime() : 0;
			return dateB - dateA;
		});

		this.cache = {
			articles,
			expiry: Date.now() + TIMING_STRATEGIES.STANDARD.interval
		};

		return { articles };
	}
}

export const GET: RequestHandler = async () => {
	try {
		const data = await RSSService.getArticles();
		return json(data);
	} catch (err) {
		console.error('RSS fetch failed', err);
		return json({ error: 'Failed to fetch RSS feeds' }, { status: 500 });
	}
};
