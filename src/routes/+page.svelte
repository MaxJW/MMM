<script lang="ts">
	import Dashboard from '$lib/components/Dashboard.svelte';
	import { buildDashboardConfig } from '$lib/config/dashboard';
	import type { DashboardConfig } from '$lib/types/dashboard';
	import type { PageData } from './$types';

	export let data: PageData;
	const dashboardConfig: DashboardConfig = buildDashboardConfig(data.dashboardUserConfig);
</script>

<Dashboard config={dashboardConfig} />
