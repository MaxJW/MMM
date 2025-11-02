<script lang="ts">
	import Dashboard from '$lib/components/Dashboard.svelte';
	import { buildDashboardConfig } from '$lib/config/dashboard';
	import type { DashboardConfig } from '$lib/types/dashboard';
	import type { PageData } from './$types';
	import { onMount, onDestroy } from 'svelte';
	import { browser } from '$app/environment';

	let { data } = $props<{ data: PageData }>();
	// Build dashboard config from props (no need for reactive state since we reload on changes)
	const dashboardConfig: DashboardConfig = buildDashboardConfig(data.dashboardUserConfig);

	let eventSource: EventSource | null = null;

	function refreshDashboardConfig() {
		console.log('Config changed, refreshing dashboard...');
		// Simply reload the page to get fresh config from server
		location.reload();
	}

	onMount(() => {
		if (!browser) return;

		// Connect to SSE stream for config change notifications
		eventSource = new EventSource('/api/config/stream');

		eventSource.addEventListener('message', (event) => {
			try {
				const data = JSON.parse(event.data);
				if (data.type === 'config-changed') {
					refreshDashboardConfig();
				}
			} catch (error) {
				console.error('Error parsing SSE message:', error);
			}
		});

		eventSource.addEventListener('error', (error) => {
			console.error('SSE connection error:', error);
			// EventSource will automatically attempt to reconnect
		});
	});

	onDestroy(() => {
		if (eventSource) {
			eventSource.close();
		}
	});
</script>

<Dashboard config={dashboardConfig} />
