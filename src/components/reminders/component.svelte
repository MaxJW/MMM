<script lang="ts">
	import Milk from '@lucide/svelte/icons/milk';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import CircleAlert from '@lucide/svelte/icons/circle-alert';
	import { onMount } from 'svelte';
	import dayjs from 'dayjs';
	import { TIMING_STRATEGIES } from '$lib/core/timing';
	import type { BinCollection } from './types';
	import { formatDate } from './api';

	let binInfo: BinCollection | null = null;
	let loading = true;
	let error: string | null = null;
	let now: dayjs.Dayjs;
	let milkReminder = { action: '', show: false };

	let timer: ReturnType<typeof setInterval> | undefined;

	function updateMilkReminder() {
		const day = now.day();
		const hour = now.hour();

		if ((day === 4 || day === 1) && hour >= 18) {
			return { action: 'Take bottles out', show: true };
		}

		if ((day === 5 || day === 2) && hour < 12) {
			return { action: 'Bring bottles in', show: true };
		}

		return { action: '', show: false };
	}

	async function loadBinData() {
		try {
			loading = true;
			error = null;
			const res = await fetch('/api/components/reminders');
			if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);

			const data = await res.json();

			if (data.error) {
				error = data.error;
				return;
			}

			binInfo = data;
			now = dayjs();
			milkReminder = updateMilkReminder();
		} catch (err) {
			error = 'Failed to load bin data';
			console.error('Error loading bin data:', err);
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		now = dayjs();
		loadBinData();
		timer = setInterval(loadBinData, TIMING_STRATEGIES.INFREQUENT.interval);
		return () => {
			if (timer) clearInterval(timer);
		};
	});
</script>

<div class="flex flex-col items-center gap-4 select-none">
	{#if loading}
		<div class="flex flex-col items-center gap-2">
			<div class="flex items-center gap-2 text-2xl opacity-80">
				<Trash2 size={22} />
				<span>Loading bin info...</span>
			</div>
		</div>
	{:else if error}
		<div class="flex flex-col items-center gap-2">
			<div class="flex items-center gap-2 text-2xl opacity-80">
				<CircleAlert size={22} />
				<span>Bin info unavailable</span>
			</div>
		</div>
	{:else if binInfo}
		<div class="flex flex-col items-center gap-2">
			<div class="flex items-center gap-2 text-2xl">
				<Trash2 size={28} class="opacity-80" />
				<span class="opacity-80">{formatDate(binInfo.date)}:</span>
				<span class="font-medium">
					{binInfo.bins.join(', ')} bin{binInfo.bins.length > 1 ? 's' : ''}
				</span>
			</div>
		</div>
	{/if}

	{#if milkReminder.show}
		<div class="flex items-center gap-3 text-2xl opacity-90">
			<Milk size={28} class="opacity-80" />
			<span>{milkReminder.action}</span>
		</div>
	{/if}
</div>
