<script lang="ts">
	import Zap from '@lucide/svelte/icons/zap';
	import { onMount } from 'svelte';
	import { EnergyService } from '$lib/services/energyService';
	import type { EnergyUsage } from '$lib/types/energy';
	import { TIMING_STRATEGIES } from '$lib/types/util';

	let usage: EnergyUsage | null = null;
	let loading = true;

	async function load() {
		usage = await EnergyService.getUsage();
		loading = false;
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
	{:else if usage}
		<div class="flex flex-col text-lg">
			<span>Today: {usage.dayKWh} kWh (£{usage.dayCost})</span>
			<span>Month: {usage.monthKWh} kWh (£{usage.monthCost})</span>
		</div>
	{/if}
</div>
