<script lang="ts">
	import Shield from '@lucide/svelte/icons/shield';
	import { onMount } from 'svelte';
	import { AdguardService } from '$lib/services/adguardService';
	import type { AdguardStats } from '$lib/types/adguard';
	import { TIMING_STRATEGIES } from '$lib/types/util';

	let stats: AdguardStats | null = null;
	let loading = true;

	async function load() {
		stats = await AdguardService.getStats();
		loading = false;
	}

	onMount(() => {
		load();
		const interval = setInterval(load, TIMING_STRATEGIES.FREQUENT.interval);
		return () => clearInterval(interval);
	});
</script>

<div class="flex flex-col items-start gap-3 select-none">
	<div class="flex items-center gap-3 opacity-90">
		<Shield size={24} />
		<h2 class="text-lg">AdGuard</h2>
	</div>
	{#if loading}
		<p>Loading…</p>
	{:else if stats}
		<div class="flex flex-col text-lg">
			<span>Queries: <span class="font-medium">{stats.queries}</span></span>
			<span>Blocked: <span class="font-medium">{stats.blocked}</span></span>
		</div>
	{/if}
</div>
