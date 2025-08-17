<script lang="ts">
	import { Milk, Trash2, CircleAlert } from 'lucide-svelte';
	import { BinService } from '$lib/services/binService';
	import { onMount } from 'svelte';
	import dayjs from 'dayjs';
	import { TIMING_STRATEGIES } from '$lib/types/util';

	let binInfo: { date: string; bins: string[] } | null = null;
	let loading = true;
	let error: string | null = null;
	let now: dayjs.Dayjs;
	let milkReminder = { action: '', show: false };

	let timer: ReturnType<typeof setInterval> | undefined;

	function updateMilkReminder() {
		const day = now.day(); // 0 = Sunday, 1 = Monday, etc.
		const hour = now.hour();

		// Thursday night (4) or Monday night (1) - put bottles out
		if ((day === 4 || day === 1) && hour >= 18) {
			return { action: 'Take bottles out', show: true };
		}

		// Friday morning (5) or Tuesday morning (2) - collect bottles
		if ((day === 5 || day === 2) && hour < 12) {
			return { action: 'Bring bottles in', show: true };
		}

		return { action: '', show: false };
	}

	async function loadBinData() {
		try {
			loading = true;
			error = null;
			binInfo = await BinService.getNextBinCollection();
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
		// Initialize now only on the client side
		now = dayjs();

		// Initial load
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
			<div class="flex items-center gap-2 text-xl opacity-70">
				<Trash2 size={22} />
				<span>Loading bin info...</span>
			</div>
		</div>
	{:else if error}
		<div class="flex flex-col items-center gap-2">
			<div class="flex items-center gap-2 text-xl opacity-70">
				<CircleAlert size={22} />
				<span>Bin info unavailable</span>
			</div>
		</div>
	{:else if binInfo}
		<div class="flex flex-col items-center gap-2">
			<div class="flex items-center gap-2 text-xl">
				<Trash2 size={28} class="opacity-80" />
				<span class="opacity-80">{BinService.formatDate(binInfo.date)}:</span>
				<span class="font-medium">
					{binInfo.bins.join(', ')} bin{binInfo.bins.length > 1 ? 's' : ''}
				</span>
			</div>
		</div>
	{/if}

	{#if milkReminder.show}
		<div class="flex items-center gap-3 text-xl opacity-80">
			<Milk size={28} class="opacity-80" />
			<span>{milkReminder.action}</span>
		</div>
	{/if}
</div>
