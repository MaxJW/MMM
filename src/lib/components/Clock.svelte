<script lang="ts">
	import { onMount } from 'svelte';
	import dayjs from 'dayjs';

	let now = dayjs();

	let timer: ReturnType<typeof setInterval> | undefined;
	onMount(() => {
		timer = setInterval(() => {
			now = dayjs();
		}, 1000);

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
		<div class="text-8xl font-semibold tabular-nums">{formatTime(now)}</div>
		<div class="pb-3 text-4xl tabular-nums opacity-70">{formatSeconds(now)}</div>
	</div>
	<div class="text-2xl opacity-80">{formatDate(now)}</div>
</div>
