<script lang="ts">
	import { onMount } from 'svelte';
	import dayjs from 'dayjs';
	import { TIMING_STRATEGIES } from '$lib/types/util';

	let now: dayjs.Dayjs;

	let timer: ReturnType<typeof setInterval> | undefined;
	onMount(() => {
		// Initialize now only on the client side
		now = dayjs();

		timer = setInterval(() => {
			now = dayjs();
		}, TIMING_STRATEGIES.REALTIME.interval);

		return () => {
			if (timer) clearInterval(timer);
		};
	});

	function formatTime(date: dayjs.Dayjs): string {
		return date.format('HH:mm');
	}

	function formatSeconds(date: dayjs.Dayjs): string {
		return date.format('ss');
	}

	function formatDate(date: dayjs.Dayjs): string {
		return date.format('dddd, D MMMM YYYY');
	}
</script>

<div class="flex flex-col items-start justify-start gap-3">
	<div class="-mt-4 flex items-end gap-4 leading-none">
		<div class="text-9xl font-semibold tabular-nums">{now ? formatTime(now) : '--:--'}</div>
		<div class="pb-3 text-5xl tabular-nums opacity-90">{now ? formatSeconds(now) : '--'}</div>
	</div>
	<div class="text-3xl opacity-90">{now ? formatDate(now) : 'Loading...'}</div>
</div>
