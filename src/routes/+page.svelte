<script lang="ts">
	import Dashboard from '$lib/components/Dashboard.svelte';
	import { buildDashboardConfig } from '$lib/config/dashboard';
	import type { DashboardConfig } from '$lib/core/types';
	import type { PageData } from './$types';
	import { onMount, onDestroy } from 'svelte';
	import { browser } from '$app/environment';

	let { data } = $props<{ data: PageData }>();

	// Build dashboard config client-side (components can't be serialized)
	let dashboardConfig: DashboardConfig | null = $state(null);

	// Load dashboard config asynchronously on client
	if (browser) {
		buildDashboardConfig(data.dashboardUserConfig).then((config) => {
			dashboardConfig = config;
		});
	}

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

{#if dashboardConfig}
	<Dashboard config={dashboardConfig} />
{:else}
	<div class="flex h-screen items-center justify-center">
		<p class="text-xl text-white">Loading dashboard...</p>
	</div>
{/if}
