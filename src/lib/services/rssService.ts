import type { Article } from '$lib/types/rss';

interface RSSApiResponse {
	articles: Article[];
}

export class RSSService {
	static async getArticles(): Promise<Article[]> {
		try {
			const response = await fetch('/api/rss-feeds');

			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			const data: RSSApiResponse = await response.json();

			if (!data || !Array.isArray(data.articles)) {
				console.warn('Invalid RSS feed data received:', data);
				return [];
			}

			return data.articles as Article[];
		} catch (error) {
			console.error('Failed to fetch RSS feed:', error);
			return [];
		}
	}
}
