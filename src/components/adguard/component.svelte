<script lang="ts">
	import Shield from '@lucide/svelte/icons/shield';
	import Globe from '@lucide/svelte/icons/globe';
	import Ban from '@lucide/svelte/icons/ban';
	import CircleAlert from '@lucide/svelte/icons/circle-alert';
	import { onMount } from 'svelte';
	import type { AdguardStats } from './types';
	import { TIMING_STRATEGIES } from '$lib/core/timing';

	let stats: AdguardStats | null = null;
	let loading = true;
	let error: string | null = null;

	async function load() {
		try {
			const res = await fetch('/api/components/adguard');
			if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);

			const data = await res.json();

			if (data.error) {
				error = data.error;
				return;
			}

			stats = data;
		} catch (err) {
			error = 'Failed to load Adguard stats';
			console.error('Error loading Adguard stats:', err);
		} finally {
			loading = false;
		}
	}

	function getBlockedPercentage(): number {
		if (!stats || stats.queries === 0) return 0;
		return Math.round((stats.blocked / stats.queries) * 100);
	}

	function formatNumber(num: number): string {
		return new Intl.NumberFormat().format(num);
	}

	onMount(() => {
		load();
		const interval = setInterval(load, TIMING_STRATEGIES.FREQUENT.interval);
		return () => clearInterval(interval);
	});
</script>

<div class="flex flex-col items-end gap-3 select-none">
	<div class="flex items-center gap-3 opacity-90">
		<Shield size={24} />
		<h2 class="text-lg">AdGuard</h2>
	</div>

	{#if loading}
		<p class="text-lg opacity-80">Loading…</p>
	{:else if error}
		<div class="flex items-center gap-2 text-lg text-red-400 opacity-80">
			<CircleAlert size={22} />
			<span>{error}</span>
		</div>
	{:else if stats}
		<div class="flex flex-col gap-2 text-lg">
			<div class="flex items-center gap-3">
				<Globe size={18} class="opacity-80" />
				<span class="text-base">Queries</span>
				<span class="font-medium tabular-nums">{formatNumber(stats.queries)}</span>
			</div>
			<div class="flex items-center gap-3">
				<Ban size={18} class="opacity-80" />
				<span class="text-base">Blocked</span>
				<span class="font-medium tabular-nums">{formatNumber(stats.blocked)}</span>
			</div>
			<div class="mt-2 flex flex-col gap-1">
				<div class="flex items-center justify-between text-sm">
					<span class="opacity-70">Block Rate</span>
					<span class="font-medium tabular-nums">{getBlockedPercentage()}%</span>
				</div>
				<!-- Progress bar showing blocked vs allowed -->
				<div class="relative h-1 overflow-hidden rounded-full bg-neutral-800">
					<!-- Blocked portion -->
					<div
						class="absolute top-0 left-0 h-full rounded-full bg-neutral-100 transition-all duration-500 ease-out"
						style="width: {getBlockedPercentage()}%"
					></div>
				</div>
			</div>
		</div>
	{/if}
</div>
