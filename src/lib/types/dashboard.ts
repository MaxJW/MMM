import type { SvelteComponent } from 'svelte';

export type DashboardArea =
	| 'top-left'
	| 'top-center'
	| 'top-right'
	| 'middle-left'
	| 'middle-right'
	| 'center'
	| 'bottom-left'
	| 'bottom-center'
	| 'bottom-right'
	| 'notifications'
	| 'custom';

export interface DashboardComponent {
	id: string;
	component: typeof SvelteComponent;
	area: DashboardArea;
	props?: Record<string, unknown>;
}

export interface DashboardConfig {
	components: DashboardComponent[];
}
