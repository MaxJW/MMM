/**
 * Component system type definitions
 */

export interface ComponentConfigField {
	key: string;
	type: 'text' | 'password' | 'number' | 'select' | 'array' | 'color';
	label: string;
	description: string;
	placeholder?: string;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	default?: any;
	options?: Array<{ value: string; label: string }>; // For select type
	itemSchema?: ComponentConfigField; // For array type - single field schema (legacy)
	itemFields?: ComponentConfigField[]; // For array type - multiple fields for object items
	hiddenWithCustomConfig?: boolean; // Hide this field when a custom config component is present
}

export interface ComponentConfigSchema {
	title: string;
	description: string;
	fields: ComponentConfigField[];
}

export interface ComponentManifest {
	id: string;
	name: string;
	version: string;
	description: string;
	config: ComponentConfigSchema;
}

export interface Component {
	id: string;
	manifest: ComponentManifest;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	component?: any; // Svelte component class
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	apiHandler?: (config: any, request?: Request) => Promise<any>; // API handler function
}
