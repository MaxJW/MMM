<script lang="ts">
	import Activity from '@lucide/svelte/icons/activity';
	import Cpu from '@lucide/svelte/icons/cpu';
	import HardDrive from '@lucide/svelte/icons/hard-drive';
	import MemoryStick from '@lucide/svelte/icons/memory-stick';
	import Thermometer from '@lucide/svelte/icons/thermometer';
	import { onMount } from 'svelte';
	import type { SystemStats } from './types';
	import { TIMING_STRATEGIES } from '$lib/core/timing';

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

			const res = await fetch('/api/components/system-stats');
			if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);

			const data = await res.json();

			if (data.error) {
				error = data.error;
				return;
			}

			stats = data;
			hasInitialData = true;
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
	<div class="flex items-center gap-3 opacity-90">
		<Activity size={24} />
		<h2 class="text-lg tracking-wide">System</h2>
	</div>

	{#if error}
		<div class="text-base text-red-400 opacity-90">
			{error}
		</div>
	{/if}

	{#if loading && !hasInitialData}
		<div class="flex flex-col items-end gap-2 text-lg opacity-80">
			<div>Loading...</div>
		</div>
	{:else}
		<div class="flex flex-col items-end gap-2 text-lg">
			<div class="flex items-center gap-3">
				<Cpu size={20} class="opacity-80" /><span>CPU</span><span class="font-medium tabular-nums"
					>{stats?.cpu ?? 0}%</span
				>
			</div>
			<div class="flex items-center gap-3">
				<MemoryStick size={20} class="opacity-80" /><span>Memory</span><span
					class="font-medium tabular-nums">{stats?.memory ?? 0}%</span
				>
			</div>
			<div class="flex items-center gap-3">
				<HardDrive size={20} class="opacity-80" /><span>Disk</span><span
					class="font-medium tabular-nums">{stats?.disk ?? 0}%</span
				>
			</div>
			<div class="flex items-center gap-3 opacity-90">
				<Thermometer size={20} class="opacity-80" /><span>Temp</span><span
					class="font-medium tabular-nums">{stats?.tempC ?? 0}°C</span
				>
			</div>
		</div>
	{/if}
</div>
