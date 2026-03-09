<script lang="ts">
	import BrokenMonitor from './effects/BrokenMonitor.svelte';
	import Eyes from './effects/Eyes.svelte';

	interface Props {
		config: { overlay?: string } | null | undefined;
	}

	let { config }: Props = $props();

	const overlay = $derived(config?.overlay ?? 'none');
</script>

{#if overlay !== 'none'}
	<div
		class="effects-overlay"
		aria-hidden="true"
	>
		{#if overlay === 'broken-monitor'}
			<BrokenMonitor />
		{:else if overlay === 'eyes'}
			<Eyes />
		{/if}
	</div>
{/if}

<style>
	.effects-overlay {
		position: fixed;
		inset: 0;
		z-index: 9999;
		pointer-events: none;
	}
</style>
