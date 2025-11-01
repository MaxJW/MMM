<script lang="ts">
	import type { DashboardConfig, DashboardArea } from '$lib/types/dashboard';

	export let config: DashboardConfig;

	const areaGridClasses: Record<DashboardArea, string> = {
		'top-left': 'absolute top-0 left-0 items-start gap-6 flex flex-col',
		'top-center': 'absolute top-0 left-1/2 -translate-x-1/2 flex items-start gap-6 flex-col',
		'top-right': 'absolute top-0 right-0 flex items-end gap-6 flex-col',

		'middle-left': 'absolute top-1/2 left-0 -translate-y-1/2 flex items-start gap-6 flex-col',
		'middle-right': 'absolute top-1/2 right-0 -translate-y-1/2 flex items-end gap-6 flex-col',
		center: 'absolute left-1/2 bottom-1/4 -translate-x-1/2 flex flex-col items-center gap-6',

		'bottom-left': 'absolute bottom-20 left-0 items-start gap-6 flex flex-col justify-end',
		'bottom-center':
			'absolute bottom-20 left-1/2 -translate-x-1/2 flex items-center gap-6 flex-col justify-end',
		'bottom-right': 'absolute bottom-20 right-0 flex items-end gap-6 flex-col',

		notifications:
			'absolute bottom-4 left-1/2 -translate-x-1/2 flex flex-col items-center gap-6 w-2/3'
	};

	function getComponentsForArea(area: DashboardArea) {
		return config.components.filter((comp) => comp.area === area);
	}
</script>

<div class="min-h-screen overflow-hidden bg-black font-bold text-neutral-100 select-none">
	<div class="relative inset-0 h-full w-full">
		<div class="grid h-screen w-full grid-cols-[1fr_1fr_1fr] grid-rows-[auto_1fr_auto_auto]">
			{#each Object.keys(areaGridClasses) as area (area)}
				<div class={areaGridClasses[area as DashboardArea]}>
					{#each getComponentsForArea(area as DashboardArea) as comp}
						<svelte:component this={comp.component} {...comp.props} />
					{/each}
				</div>
			{/each}
		</div>
	</div>
</div>
