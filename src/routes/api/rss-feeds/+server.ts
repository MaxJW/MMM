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

	private static async getFeeds(): Promise<FeedConfig[]> {
		const { getRSSFeedsConfig } = await import('$lib/config/userConfig');
		const feeds = await getRSSFeedsConfig();
		return feeds.map((feed) => ({
			url: feed.url,
			sourceName: feed.sourceName
		}));
	}

	private static decodeNumericEntities(str: string): string {
		if (!str) return '';
		const entityMap: Record<string, string> = {
			'8211': '–', // en dash
			'8216': '‘', // left single quote
			'8217': '’', // right single quote
			'8230': '…', // ellipsis
			'8242': '″', // double prime
			'8243': '‴', // triple prime
			'8259': '⁠', // non-breaking space
			'8260': '⁄', // fraction slash
			'8261': '⁼', // equals to
			'8262': '⁺', // plus sign
			'8263': '⁻', // minus sign
			'8264': '⁰', // zero
			'8265': 'ⁱ', // i
			'8266': '⁴', // four
			'8267': '⁵', // five
			'8268': '⁶', // six
			'8269': '⁷', // seven
			'8270': '⁸', // eight
			'8271': '⁹', // nine
			'163': '£', // pound sign
			'169': '©', // copyright sign
			'174': '®', // registered trademark
			'8482': '™', // trademark
			'8220': '“', // left double quote
			'8221': '”', // right double quote
			'8226': '•', // bullet
			'38': '&', // ampersand
			'60': '<', // less than
			'62': '>', // greater than
			'34': '"', // double quote
			'39': "'", // single quote
			'8212': '—', // em dash
			'8218': '‚', // single low-9 quotation mark
			'8222': '„', // double low-9 quotation mark
			'8232': '‒', // figure dash
			'183': '·' // middle dot
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

		const feeds = await this.getFeeds();
		const allArticles = await Promise.all(feeds.map((feed) => this.fetchFeed(feed)));
		const articles = allArticles.flat();

		// Calculate the date one week ago
		const oneWeekAgo = new Date();
		oneWeekAgo.setDate(oneWeekAgo.getDate() - 7);

		// Filter to only include articles from the past week
		const recentArticles = articles.filter((article) => {
			if (!article.date) return false; // Exclude articles without dates
			const articleDate = new Date(article.date);
			return articleDate >= oneWeekAgo;
		});

		// Sort newest → oldest
		recentArticles.sort((a, b) => {
			const dateA = a.date ? new Date(a.date).getTime() : 0;
			const dateB = b.date ? new Date(b.date).getTime() : 0;
			return dateB - dateA;
		});

		this.cache = {
			articles: recentArticles,
			expiry: Date.now() + TIMING_STRATEGIES.STANDARD.interval
		};

		return { articles: recentArticles };
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
