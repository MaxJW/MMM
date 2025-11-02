<script lang="ts">
	import { onMount } from 'svelte';
	import Wifi from '@lucide/svelte/icons/wifi';

	let qrCodeUrl: string | null = null;
	let networkName: string = '';
	let loading = true;

	onMount(async () => {
		try {
			const response = await fetch('/api/wifi/qrcode');
			if (response.ok) {
				const data = await response.json();
				qrCodeUrl = data.qrCode;
				networkName = data.networkName || '';
			}
		} catch (error) {
			console.error('Failed to load WiFi QR code:', error);
		} finally {
			loading = false;
		}
	});
</script>

{#if !loading && networkName}
	<div class="flex flex-col items-start gap-3 select-none">
		<div class="flex items-center gap-3 opacity-90">
			<Wifi size={24} />
			<h2 class="text-lg">WiFi Details</h2>
		</div>

		{#if qrCodeUrl}
			<img class="size-40" src={qrCodeUrl} alt="Wifi QR Code" />
		{/if}

		<div class="text-base opacity-90">
			<span class="font-extrabold">Network:</span>
			{networkName}
		</div>
	</div>
{/if}
