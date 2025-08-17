// src/routes/api/rss/+server.ts
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { XMLParser } from 'fast-xml-parser';
import type { Article } from '$lib/types/rss';
import { THIRTY_MIN } from '$lib/types/util';

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
			url: 'https://feeds.bbci.co.uk/news/rss.xml',
			sourceName: 'BBC News'
		}
	];

	static async fetchFeed(feed: FeedConfig): Promise<Article[]> {
		const res = await fetch(feed.url);
		if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
		const xmlText = await res.text();

		const parser = new XMLParser({ ignoreAttributes: false, attributeNamePrefix: '' });
		const data = parser.parse(xmlText);

		const channel = data.rss?.channel || data.feed;
		const feedTitle = feed.sourceName || channel?.title || 'RSS Feed';

		const items = Array.isArray(channel?.item) ? channel.item : channel?.item ? [channel.item] : [];

		return items.map((item: ParsedRSSItem) => ({
			title: item.title || 'Untitled',
			source: feedTitle,
			date: item.pubDate ? new Date(item.pubDate).toISOString() : ''
		}));
	}

	static async getArticles(): Promise<{ articles: Article[] }> {
		if (this.cache && Date.now() < this.cache.expiry) return { articles: this.cache.articles };

		const allArticles = await Promise.all(this.feeds.map((feed) => this.fetchFeed(feed)));
		const articles = allArticles.flat();

		this.cache = {
			articles,
			expiry: Date.now() + THIRTY_MIN // cache for 30 minutes
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
