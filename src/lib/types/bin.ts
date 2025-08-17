export interface BinCollection {
	date: string;
	bins: string[];
}

export interface BinApiResponse {
	data?: {
		tab_collections?: {
			date: string;
			colour: string;
		}[];
	};
}
