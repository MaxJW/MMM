<script lang="ts">
	import Zap from '@lucide/svelte/icons/zap';
	import { onMount } from 'svelte';
	import type { EnergyUsage } from './types';
	import { TIMING_STRATEGIES } from '$lib/core/timing';

	let usage: EnergyUsage | null = null;
	let loading = true;
	let error: string | null = null;

	async function load() {
		try {
			const res = await fetch('/api/components/energy');
			if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);

			const data = await res.json();

			if (data.error) {
				error = data.error;
				return;
			}

			usage = data;
		} catch (err) {
			error = 'Failed to load energy usage';
			console.error('Error loading energy usage:', err);
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		load();
		const interval = setInterval(load, TIMING_STRATEGIES.STANDARD.interval);
		return () => clearInterval(interval);
	});
</script>

<div class="flex flex-col items-start gap-3 select-none">
	<div class="flex items-center gap-3 opacity-90">
		<Zap size={24} />
		<h2 class="text-lg">Energy</h2>
	</div>
	{#if loading}
		<p>Loading…</p>
	{:else if error}
		<p class="text-red-400 opacity-80">{error}</p>
	{:else if usage}
		<div class="flex flex-col text-lg">
			<span>Today: {usage.dayKWh} kWh (£{usage.dayCost})</span>
			<span>Month: {usage.monthKWh} kWh (£{usage.monthCost})</span>
		</div>
	{/if}
</div>
