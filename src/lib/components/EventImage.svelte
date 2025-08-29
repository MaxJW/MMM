<script>
	import EventImg from '$lib/assets/EventImg.png';
	import QRCode from '$lib/assets/EventQRCode.png';
</script>

<div class="flex flex-col items-center gap-16 select-none">
	<img src={EventImg} alt="Event" />
	<div class="flex flex-col items-center">
		<h2 class="text-xl">Join the POV event below!</h2>
		<img class="size-60" src={QRCode} alt="Wifi QR Code" />
	</div>
</div>
