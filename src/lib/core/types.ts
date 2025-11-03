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
	| 'notifications';

export interface DashboardComponent {
	id: string;
	component: any; // Svelte component class - using any due to dynamic loading
	area: DashboardArea;
	enabled: boolean;
	props?: Record<string, unknown>;
}

export interface DashboardConfig {
	components: DashboardComponent[];
}
