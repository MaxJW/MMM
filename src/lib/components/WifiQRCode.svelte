<script lang="ts">
	import QRCode from '$lib/assets/QRCode.png';
	import Wifi from '@lucide/svelte/icons/wifi';
</script>

<div class="flex flex-col items-start gap-3 select-none">
	<div class="flex items-center gap-3 opacity-90">
		<Wifi size={24} />
		<h2 class="text-lg">WiFi Details</h2>
	</div>

	<img class="size-40" src={QRCode} alt="Wifi QR Code" />

	<div class="text-base opacity-90">
		<span class="font-extrabold">Network:</span> VM6417600
	</div>
</div>
