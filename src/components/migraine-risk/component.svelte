<script lang="ts">
	import { onMount } from 'svelte';
	import AlertTriangle from '@lucide/svelte/icons/alert-triangle';
	import { TIMING_STRATEGIES } from '$lib/core/timing';
	import { resolveMigraineRiskUiOptions, type MigraineRiskComponentConfig } from './config';
	import type { FactorBreakdown, MigraineRiskPayload, RiskLevelLabel } from './types';
	import { MIGRAINE_MAX_SCORE } from './types';

	export let config: MigraineRiskComponentConfig | undefined = undefined;

	let payload: MigraineRiskPayload | null = null;
	let loading = true;
	let error: string | null = null;
	let hasInitialData = false;

	$: opts = resolveMigraineRiskUiOptions(config);

	function levelAccent(level: RiskLevelLabel): string {
		switch (level) {
			case 'Low':
				return 'text-emerald-400';
			case 'Moderate':
				return 'text-amber-300';
			case 'High':
				return 'text-orange-400';
			case 'Very High':
				return 'text-red-400';
		}
	}

	function levelStroke(level: RiskLevelLabel): string {
		switch (level) {
			case 'Low':
				return '#34d399';
			case 'Moderate':
				return '#fcd34d';
			case 'High':
				return '#fb923c';
			case 'Very High':
				return '#f87171';
		}
	}

	function factorEntries(f: FactorBreakdown): Array<{ key: string; label: string; pts: number }> {
		if (!payload) return [];
		const labels = payload.factorLabels;
		return [
			{ key: 'pressure6h', label: labels.pressure6h, pts: f.pressure6h },
			{ key: 'pressure24h', label: labels.pressure24h, pts: f.pressure24h },
			{ key: 'humidity', label: labels.humidity, pts: f.humidity },
			{ key: 'temperature', label: labels.temperature, pts: f.temperature },
			{ key: 'temperatureChange', label: labels.temperatureChange, pts: f.temperatureChange },
			{ key: 'wind', label: labels.wind, pts: f.wind },
			{ key: 'uv', label: labels.uv, pts: f.uv },
			{ key: 'thunderstorm', label: labels.thunderstorm, pts: f.thunderstorm },
			{ key: 'airQuality', label: labels.airQuality, pts: f.airQuality }
		];
	}

	function displayedFactors(
		f: FactorBreakdown
	): Array<{ key: string; label: string; pts: number }> {
		const rows = factorEntries(f);
		if (opts.onlyContributingFactors) return rows.filter((r) => r.pts > 0);
		return rows;
	}

	async function loadRisk() {
		try {
			if (!hasInitialData) loading = true;
			error = null;
			const res = await fetch('/api/components/migraine-risk');
			if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
			const data = (await res.json()) as MigraineRiskPayload & { error?: string };
			if (data.error) {
				error = data.error;
				return;
			}
			payload = data;
			hasInitialData = true;
		} catch (err) {
			error = 'Failed to load migraine risk';
			console.error('migraine-risk:', err);
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		loadRisk();
		const interval = setInterval(loadRisk, TIMING_STRATEGIES.MEDIUM.interval);
		return () => clearInterval(interval);
	});

	$: gaugePct = payload ? Math.min(100, (payload.score / MIGRAINE_MAX_SCORE) * 100) : 0;
	$: strokeColor = payload ? levelStroke(payload.level) : '#525252';
</script>

<div class="flex max-w-md flex-col items-end gap-3 select-none">
	<div class="flex items-center gap-2 text-neutral-100 opacity-90">
		<AlertTriangle size={opts.compactLayout ? 18 : 22} class="flex-shrink-0" />
		<h2 class="{opts.compactLayout ? 'text-base' : 'text-lg'} font-bold tracking-wide">
			Migraine risk
		</h2>
	</div>

	{#if error}
		<p class="max-w-xs text-right text-base text-red-400 opacity-90">{error}</p>
	{/if}

	{#if loading && !hasInitialData}
		<p class="text-lg opacity-80">Loading…</p>
	{:else if payload}
		<div class="flex flex-col items-end gap-1">
			{#if opts.showGauge}
				<div class="relative {opts.compactLayout ? 'h-[4.5rem] w-48' : 'h-[5.5rem] w-56'}">
					<svg viewBox="0 0 120 72" class="h-full w-full overflow-visible" aria-hidden="true">
						<path
							d="M 12 60 A 48 48 0 0 1 108 60"
							fill="none"
							stroke="rgb(38 38 38)"
							stroke-width="8"
							stroke-linecap="round"
						/>
						<path
							d="M 12 60 A 48 48 0 0 1 108 60"
							fill="none"
							stroke={strokeColor}
							stroke-width="8"
							stroke-linecap="round"
							pathLength="100"
							stroke-dasharray="100"
							stroke-dashoffset={100 - gaugePct}
							class="transition-[stroke-dashoffset] duration-700 ease-out"
						/>
					</svg>
					<div
						class="absolute bottom-0 left-1/2 flex -translate-x-1/2 flex-col items-center leading-none"
					>
						<span class="{opts.compactLayout ? 'text-4xl' : 'text-5xl'} font-semibold tabular-nums"
							>{payload.score}</span
						>
						{#if opts.showMaxScore}
							<span class="text-sm opacity-70">/ {MIGRAINE_MAX_SCORE}</span>
						{/if}
					</div>
				</div>
			{:else}
				<div class="flex flex-col items-end gap-0 leading-none">
					<div class="flex items-baseline gap-2">
						<span class="{opts.compactLayout ? 'text-4xl' : 'text-5xl'} font-semibold tabular-nums"
							>{payload.score}</span
						>
						{#if opts.showMaxScore}
							<span class="{opts.compactLayout ? 'text-base' : 'text-lg'} opacity-70"
								>/ {MIGRAINE_MAX_SCORE}</span
							>
						{/if}
					</div>
				</div>
			{/if}
			<p
				class="{opts.compactLayout ? 'text-xl' : 'text-2xl'} font-bold {levelAccent(payload.level)}"
			>
				{payload.level}
			</p>
		</div>

		{#if opts.showAirQualityNote && !payload.meta.airQualityAvailable}
			<div class="mt-1 w-full text-right text-sm opacity-80">
				<span class="block text-xs opacity-60">Air quality: not available (standalone)</span>
			</div>
		{/if}

		{#if opts.showBreakdown}
			<ul class="mt-2 w-full space-y-1 text-right {opts.compactLayout ? 'text-sm' : 'text-base'}">
				{#each displayedFactors(payload.factors) as row (row.key)}
					<li
						class="flex justify-end gap-4 tabular-nums {row.pts > 0
							? 'font-medium text-neutral-100'
							: 'text-neutral-500'}"
					>
						<span class="max-w-[11rem] truncate opacity-90">{row.label}</span>
						<span class="min-w-[2ch]">{row.pts}</span>
					</li>
				{:else}
					{#if opts.onlyContributingFactors}
						<li class="text-neutral-500 opacity-80">No contributing factors right now</li>
					{/if}
				{/each}
			</ul>
		{/if}

		{#if opts.showTomorrow && payload.tomorrow.available && payload.tomorrow.score != null && payload.tomorrow.level}
			<div class="mt-3 w-full border-t border-neutral-800 pt-3 text-right">
				<div class="text-xs font-semibold tracking-wider uppercase opacity-60">Tomorrow</div>
				<div class="flex flex-col items-end gap-0.5">
					<span class="{opts.compactLayout ? 'text-lg' : 'text-xl'} font-semibold tabular-nums"
						>{payload.tomorrow.score} pts</span
					>
					<span
						class="{opts.compactLayout ? 'text-base' : 'text-lg'} font-bold {levelAccent(
							payload.tomorrow.level
						)}">{payload.tomorrow.level}</span
					>
				</div>
			</div>
		{/if}
	{/if}
</div>
