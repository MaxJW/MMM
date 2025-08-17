<script lang="ts">
	import { Activity, Cpu, HardDrive, MemoryStick, Thermometer } from 'lucide-svelte';
	import { onMount } from 'svelte';
	import { SystemStatsService } from '$lib/services/systemStatsService';
	import type { SystemStats } from '$lib/types/system-stats';
	import { TIMING_STRATEGIES } from '$lib/types/util';

	let stats: SystemStats | null = null;
	let loading = true;
	let error: string | null = null;
	let hasInitialData = false;

	async function loadSystemStats() {
		try {
			if (!hasInitialData) {
				loading = true;
			}
			error = null;

			const data = await SystemStatsService.getSystemStats();

			if (data) {
				stats = data;
				hasInitialData = true;
			} else {
				throw new Error('No data received');
			}
		} catch (err) {
			error = 'Failed to load system stats';
			console.error('Error loading system stats:', err);
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		loadSystemStats();

		const interval = setInterval(loadSystemStats, TIMING_STRATEGIES.FREQUENT.interval);

		return () => clearInterval(interval);
	});
</script>

<div class="flex flex-col items-end gap-3 select-none">
	<div class="flex items-center gap-3 opacity-80">
		<Activity size={24} />
		<h2 class="text-base tracking-wide uppercase">System</h2>
	</div>

	{#if error}
		<div class="text-sm text-red-400 opacity-80">
			{error}
		</div>
	{/if}

	{#if loading && !hasInitialData}
		<div class="flex flex-col items-end gap-2 text-base opacity-70">
			<div>Loading...</div>
		</div>
	{:else}
		<div class="flex flex-col items-end gap-2 text-base">
			<div class="flex items-center gap-3">
				<Cpu size={20} class="opacity-70" /><span>CPU</span><span class="font-medium tabular-nums"
					>{stats?.cpu ?? 0}%</span
				>
			</div>
			<div class="flex items-center gap-3">
				<MemoryStick size={20} class="opacity-70" /><span>Memory</span><span
					class="font-medium tabular-nums">{stats?.memory ?? 0}%</span
				>
			</div>
			<div class="flex items-center gap-3">
				<HardDrive size={20} class="opacity-70" /><span>Disk</span><span
					class="font-medium tabular-nums">{stats?.disk ?? 0}%</span
				>
			</div>
			<div class="flex items-center gap-3 opacity-80">
				<Thermometer size={20} class="opacity-70" /><span>Temp</span><span class="tabular-nums"
					>{stats?.tempC ?? 0}°C</span
				>
			</div>
		</div>
	{/if}
</div>
