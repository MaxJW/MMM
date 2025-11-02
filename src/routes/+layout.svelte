<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import { page } from '$app/stores';

	let { children } = $props();

	$effect(() => {
		const isHomepage = $page.url.pathname === '/';
		document.documentElement.style.cursor = isHomepage ? 'none' : 'default';
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children?.()}
