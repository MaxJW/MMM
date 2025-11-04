<script lang="ts">
	import type { Icon } from '@lucide/svelte';
	import Milk from '@lucide/svelte/icons/milk';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import CircleAlert from '@lucide/svelte/icons/circle-alert';
	import Bell from '@lucide/svelte/icons/bell';
	import Calendar from '@lucide/svelte/icons/calendar';
	import Clock from '@lucide/svelte/icons/clock';
	import Home from '@lucide/svelte/icons/home';
	import ShoppingCart from '@lucide/svelte/icons/shopping-cart';
	import Utensils from '@lucide/svelte/icons/utensils';
	import Package from '@lucide/svelte/icons/package';
	import Heart from '@lucide/svelte/icons/heart';
	import Star from '@lucide/svelte/icons/star';
	import CheckCircle from '@lucide/svelte/icons/check-circle';
	import AlertCircle from '@lucide/svelte/icons/alert-circle';
	import { onMount } from 'svelte';
	import dayjs from 'dayjs';
	import { TIMING_STRATEGIES } from '$lib/core/timing';
	import type { BinCollection } from './types';
	import { formatDate } from './api';

	interface ReminderConfig {
		text: string;
		icon?: string;
		days?: number[] | string; // 0 = Sunday, 1 = Monday, ..., 6 = Saturday (can be array or comma-separated string)
		startHour?: number | null; // 0-23
		endHour?: number | null; // 0-23
	}

	interface RemindersComponentConfig {
		reminders?: ReminderConfig[];
	}

	// Icon map for dynamic icon selection
	const iconMap: Record<string, typeof Icon> = {
		milk: Milk,
		bell: Bell,
		calendar: Calendar,
		clock: Clock,
		home: Home,
		'shopping-cart': ShoppingCart,
		utensils: Utensils,
		package: Package,
		heart: Heart,
		star: Star,
		'check-circle': CheckCircle,
		'alert-circle': AlertCircle,
		trash: Trash2
	};

	let binInfo: BinCollection | null = null;
	let loading = true;
	let error: string | null = null;
	let now: dayjs.Dayjs;
	let activeReminders: Array<{ text: string; icon: typeof Icon }> = [];
	let remindersConfig: RemindersComponentConfig = {};

	let timer: ReturnType<typeof setInterval> | undefined;

	function getIcon(iconName?: string): typeof Icon {
		if (!iconName) return Bell; // Default icon
		return iconMap[iconName.toLowerCase()] || Bell;
	}

	function parseDays(days: number[] | string | undefined): number[] | undefined {
		if (!days) return undefined;
		if (Array.isArray(days)) return days;
		if (typeof days === 'string') {
			// Parse comma-separated string
			const parsed = days
				.split(',')
				.map((d) => parseInt(d.trim(), 10))
				.filter((d) => !isNaN(d) && d >= 0 && d <= 6);
			return parsed.length > 0 ? parsed : undefined;
		}
		return undefined;
	}

	function checkReminder(reminder: ReminderConfig): boolean {
		const day = now.day();
		const hour = now.hour();

		// Check day of week
		const daysArray = parseDays(reminder.days);
		if (daysArray && daysArray.length > 0) {
			if (!daysArray.includes(day)) {
				return false;
			}
		}

		// Check hour range
		const startHour = reminder.startHour;
		const endHour = reminder.endHour;

		if (
			startHour !== undefined &&
			startHour !== null &&
			endHour !== undefined &&
			endHour !== null
		) {
			// Both start and end specified - check if hour is in range
			if (startHour <= endHour) {
				// Normal range (e.g., 18-23 means hour >= 18 && hour < 23)
				if (hour < startHour || hour >= endHour) {
					return false;
				}
			} else {
				// Wraps around midnight (e.g., 22-6 means hour >= 22 || hour < 6)
				if (hour < startHour && hour >= endHour) {
					return false;
				}
			}
		} else if (startHour !== undefined && startHour !== null) {
			// Only startHour specified - means "from startHour to end of day"
			if (hour < startHour) {
				return false;
			}
		} else if (endHour !== undefined && endHour !== null) {
			// Only endHour specified - means "from start of day to endHour"
			if (hour >= endHour) {
				return false;
			}
		}

		return true;
	}

	function updateReminders() {
		activeReminders = [];
		if (!remindersConfig.reminders || remindersConfig.reminders.length === 0) {
			return;
		}

		for (const reminder of remindersConfig.reminders) {
			if (checkReminder(reminder)) {
				activeReminders.push({
					text: reminder.text,
					icon: getIcon(reminder.icon)
				});
			}
		}
	}

	async function loadConfig() {
		try {
			const res = await fetch('/api/config');
			if (res.ok) {
				const config = await res.json();
				remindersConfig = (config.components?.reminders as RemindersComponentConfig) ?? {};
			}
		} catch (err) {
			console.error('Failed to load reminders config:', err);
			remindersConfig = {};
		}
	}

	async function loadBinData() {
		try {
			loading = true;
			error = null;
			const res = await fetch('/api/components/reminders');
			if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);

			const data = await res.json();

			if (data === null) {
				binInfo = null;
			} else if (data && typeof data === 'object' && 'error' in data) {
				error = data.error;
				binInfo = null;
			} else {
				binInfo = data;
			}
		} catch (err) {
			error = 'Failed to load bin data';
			binInfo = null;
			console.error('Error loading bin data:', err);
		} finally {
			// Always update time and reminders regardless of bin data success/failure
			now = dayjs();
			updateReminders();
			loading = false;
		}
	}

	onMount(() => {
		now = dayjs();
		loadConfig()
			.then(() => {
				updateReminders();
				loadBinData();
			})
			.catch(console.error);

		timer = setInterval(() => {
			now = dayjs();
			updateReminders();
			loadBinData();
		}, TIMING_STRATEGIES.INFREQUENT.interval);
		return () => {
			if (timer) clearInterval(timer);
		};
	});
</script>

<div class="flex flex-col items-center gap-4 select-none">
	{#if loading || error || binInfo}
		<div class="flex items-center gap-2 text-2xl {loading || error ? 'opacity-80' : ''}">
			{#if loading}
				<Trash2 size={22} />
				<span>Loading bin info...</span>
			{:else if error}
				<CircleAlert size={22} />
				<span>Bin info unavailable</span>
			{:else if binInfo}
				<Trash2 size={28} class="opacity-80" />
				<span class="opacity-80">{formatDate(binInfo.date)}:</span>
				<span class="font-medium">
					{binInfo.bins.join(', ')} bin{binInfo.bins.length > 1 ? 's' : ''}
				</span>
			{/if}
		</div>
	{/if}

	{#each activeReminders as reminder}
		<div class="flex items-center gap-3 text-2xl opacity-90">
			<svelte:component this={reminder.icon} size={28} class="opacity-80" />
			<span>{reminder.text}</span>
		</div>
	{/each}
</div>
