<script lang="ts">
	import { onMount } from 'svelte';
	import Wifi from '@lucide/svelte/icons/wifi';
	import CircleAlert from '@lucide/svelte/icons/circle-alert';

	let qrCodeUrl: string | null = null;
	let networkName: string = '';
	let loading = true;
	let error: string | null = null;

	onMount(async () => {
		try {
			loading = true;
			error = null;
			const response = await fetch('/api/components/wifi-qr-code');
			if (!response.ok) throw new Error(`HTTP error! status: ${response.status}`);

			const data = await response.json();
			if (data.error) {
				error = data.error;
				return;
			}
			qrCodeUrl = data.qrCode;
			networkName = data.networkName || '';
		} catch (err) {
			error = 'Failed to load WiFi QR code';
			console.error('Failed to load WiFi QR code:', err);
		} finally {
			loading = false;
		}
	});
</script>

<div class="flex flex-col items-start gap-3 select-none">
	<div class="flex items-center gap-3 opacity-90">
		<Wifi size={24} />
		<h2 class="text-lg">WiFi Details</h2>
	</div>

	{#if loading}
		<p class="text-lg opacity-80">Loading…</p>
	{:else if error}
		<div class="flex items-center gap-2 text-lg text-red-400 opacity-80">
			<CircleAlert size={22} />
			<span>{error}</span>
		</div>
	{:else if networkName}
		{#if qrCodeUrl}
			<img class="size-40" src={qrCodeUrl} alt="Wifi QR Code" />
		{/if}

		<div class="text-base opacity-90">
			<span class="font-extrabold">Network:</span>
			{networkName}
		</div>
	{:else}
		<p class="text-lg opacity-80">No WiFi network configured</p>
	{/if}
</div>
