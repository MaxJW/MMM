import { readFile, writeFile, mkdir } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';

const TOKEN_DIR = join(process.cwd(), 'data');
const TOKEN_FILE = join(TOKEN_DIR, 'google-tokens.json');

interface TokenData {
	refreshToken: string;
	accessToken?: string;
	expiryDate?: number;
}

export class TokenStorage {
	static async init() {
		if (!existsSync(TOKEN_DIR)) {
			await mkdir(TOKEN_DIR, { recursive: true });
		}
	}

	static async saveTokens(tokens: TokenData) {
		await this.init();
		await writeFile(TOKEN_FILE, JSON.stringify(tokens, null, 2));
	}

	static async loadTokens(): Promise<TokenData | null> {
		try {
			if (!existsSync(TOKEN_FILE)) return null;
			const data = await readFile(TOKEN_FILE, 'utf8');
			return JSON.parse(data);
		} catch (error) {
			console.error('Error loading tokens:', error);
			return null;
		}
	}
}
