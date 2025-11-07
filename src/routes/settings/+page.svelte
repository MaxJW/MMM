<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import type { UserConfig } from '$lib/core/config';
	import Settings from '@lucide/svelte/icons/settings';
	import Save from '@lucide/svelte/icons/save';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import CheckCircle2 from '@lucide/svelte/icons/check-circle-2';
	import AlertCircle from '@lucide/svelte/icons/alert-circle';
	import ChevronUp from '@lucide/svelte/icons/chevron-up';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import type { ComponentManifest } from '$lib/components/types';
	import ComponentField from '$lib/components/ComponentField.svelte';
	import type { DashboardArea } from '$lib/core/types';

	let config: UserConfig | null = $state(null);
	let originalConfig: UserConfig | null = $state(null);
	let loading = $state(true);
	let saving = $state(false);
	let saveMessage: { type: 'success' | 'error'; text: string } | null = $state(null);
	let activeTab = $state('dashboard');
	let componentManifests: ComponentManifest[] = $state([]);
	let tabs: Array<{ id: string; label: string }> = $state([
		{ id: 'dashboard', label: 'Dashboard Layout' }
	]);
	let configComponentMap: Record<string, unknown> = $state({});

	// Area order for grouping components
	const areaOrder: DashboardArea[] = [
		'top-left',
		'top-center',
		'top-right',
		'middle-left',
		'middle-right',
		'center',
		'bottom-left',
		'bottom-center',
		'bottom-right',
		'notifications'
	];

	const areaLabels: Record<DashboardArea, string> = {
		'top-left': 'Top Left',
		'top-center': 'Top Center',
		'top-right': 'Top Right',
		'middle-left': 'Middle Left',
		'middle-right': 'Middle Right',
		center: 'Center',
		'bottom-left': 'Bottom Left',
		'bottom-center': 'Bottom Center',
		'bottom-right': 'Bottom Right',
		notifications: 'Notifications'
	};

	function getComponentsByArea() {
		if (!config) return {};

		const groups: Record<string, Array<{ comp: any; index: number }>> = {};

		config.dashboard.components.forEach((comp, index) => {
			if (!groups[comp.area]) {
				groups[comp.area] = [];
			}
			groups[comp.area].push({ comp, index });
		});

		return groups;
	}

	function getAreaLabel(area: DashboardArea): string {
		return areaLabels[area] || area;
	}

	// Password visibility states
	let passwordVisibility: Record<string, boolean> = $state({});

	function togglePasswordVisibility(fieldId: string) {
		passwordVisibility[fieldId] = !(passwordVisibility[fieldId] ?? false);
	}

	onMount(async () => {
		try {
			const [configResponse, manifestsResponse, configComponentsResponse] = await Promise.all([
				fetch('/api/config'),
				fetch('/api/components/manifests'),
				import('../../components/config-handlers')
			]);

			if (configResponse.ok) {
				const loadedConfig = await configResponse.json();
				config = loadedConfig;
				// Store a deep copy of the original config for comparison
				originalConfig = JSON.parse(JSON.stringify(loadedConfig));
			}

			if (manifestsResponse.ok) {
				const data = await manifestsResponse.json();
				if (data.components) {
					componentManifests = data.components.map(
						(c: { manifest: ComponentManifest }) => c.manifest
					);
					const componentTabs = componentManifests
						.map((m) => ({ id: m.id, label: m.name }))
						.sort((a, b) => a.label.localeCompare(b.label));
					tabs = [{ id: 'dashboard', label: 'Dashboard Layout' }, ...componentTabs];
				}
			}

			// Load config components map
			if (configComponentsResponse.configComponentMap) {
				configComponentMap = configComponentsResponse.configComponentMap;
			}
		} catch (error) {
			console.error('Failed to load configuration:', error);
		} finally {
			loading = false;
		}
	});

	function updateComponentConfig(componentId: string, key: string, value: any) {
		if (!config) return;

		if (!config.components[componentId]) {
			config.components[componentId] = {};
		}

		config.components[componentId][key] = value;
		config = { ...config };
	}

	function getComponentConfig(componentId: string): Record<string, any> {
		return config?.components[componentId] || {};
	}

	async function saveConfig() {
		if (!config) return;

		saving = true;
		saveMessage = null;

		try {
			const configResponse = await fetch('/api/config', {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(config)
			});

			if (!configResponse.ok) {
				const data = await configResponse.json();
				saveMessage = { type: 'error', text: data.error || 'Failed to save configuration' };
				saving = false;
				return;
			}

			saveMessage = { type: 'success', text: 'Configuration saved successfully!' };
			// Update original config after successful save
			originalConfig = JSON.parse(JSON.stringify(config));
			setTimeout(() => {
				saveMessage = null;
			}, 3000);
		} catch (error) {
			saveMessage = { type: 'error', text: 'Failed to save configuration' };
		} finally {
			saving = false;
		}
	}

	function handleAreaChange(componentIndex: number, newArea: DashboardArea) {
		if (!config) return;

		const component = config.dashboard.components[componentIndex];
		const oldArea = component.area;

		if (oldArea === newArea) return;

		// Update the component's area
		component.area = newArea;

		// Remove component from current position
		const [movedComponent] = config.dashboard.components.splice(componentIndex, 1);

		// Find where to insert it in the new area (at the end of that area's components)
		let insertIndex = config.dashboard.components.length;
		for (let i = 0; i < config.dashboard.components.length; i++) {
			const currentArea = config.dashboard.components[i].area;
			const currentAreaOrder = areaOrder.indexOf(currentArea);
			const newAreaOrder = areaOrder.indexOf(newArea);

			if (currentAreaOrder > newAreaOrder) {
				insertIndex = i;
				break;
			}
		}

		// Insert the component at the calculated position
		config.dashboard.components.splice(insertIndex, 0, movedComponent);

		// Trigger reactivity
		config = { ...config };
	}

	function moveComponentUp(globalIndex: number) {
		if (!config) return;

		const component = config.dashboard.components[globalIndex];
		const area = component.area;

		// Find all components in the same area
		const componentsInArea = config.dashboard.components
			.map((c, i) => ({ component: c, index: i }))
			.filter((item) => item.component.area === area);

		// Find position in area
		const positionInArea = componentsInArea.findIndex((item) => item.index === globalIndex);

		// Can't move up if already first in area
		if (positionInArea === 0) return;

		// Get the component above in the same area
		const targetIndex = componentsInArea[positionInArea - 1].index;

		// Swap the two components
		[config.dashboard.components[globalIndex], config.dashboard.components[targetIndex]] = [
			config.dashboard.components[targetIndex],
			config.dashboard.components[globalIndex]
		];

		// Trigger reactivity
		config = { ...config };
	}

	function moveComponentDown(globalIndex: number) {
		if (!config) return;

		const component = config.dashboard.components[globalIndex];
		const area = component.area;

		// Find all components in the same area
		const componentsInArea = config.dashboard.components
			.map((c, i) => ({ component: c, index: i }))
			.filter((item) => item.component.area === area);

		// Find position in area
		const positionInArea = componentsInArea.findIndex((item) => item.index === globalIndex);

		// Can't move down if already last in area
		if (positionInArea === componentsInArea.length - 1) return;

		// Get the component below in the same area
		const targetIndex = componentsInArea[positionInArea + 1].index;

		// Swap the two components
		[config.dashboard.components[globalIndex], config.dashboard.components[targetIndex]] = [
			config.dashboard.components[targetIndex],
			config.dashboard.components[globalIndex]
		];

		// Trigger reactivity
		config = { ...config };
	}

	function canMoveUp(globalIndex: number): boolean {
		if (!config) return false;

		const component = config.dashboard.components[globalIndex];
		const area = component.area;

		const componentsInArea = config.dashboard.components
			.map((c, i) => ({ component: c, index: i }))
			.filter((item) => item.component.area === area);

		const positionInArea = componentsInArea.findIndex((item) => item.index === globalIndex);

		return positionInArea > 0;
	}

	function canMoveDown(globalIndex: number): boolean {
		if (!config) return false;

		const component = config.dashboard.components[globalIndex];
		const area = component.area;

		const componentsInArea = config.dashboard.components
			.map((c, i) => ({ component: c, index: i }))
			.filter((item) => item.component.area === area);

		const positionInArea = componentsInArea.findIndex((item) => item.index === globalIndex);

		return positionInArea < componentsInArea.length - 1;
	}

	function hasUnsavedChanges(): boolean {
		if (!config || !originalConfig) return false;
		return JSON.stringify(config) !== JSON.stringify(originalConfig);
	}

	function handleBackToMirror() {
		if (hasUnsavedChanges()) {
			const confirmed = confirm(
				'You have unsaved changes. Are you sure you want to leave? Your changes will be lost.'
			);
			if (!confirmed) {
				return;
			}
		}
		goto('/');
	}

	// Handle OAuth callback messages
	$effect(() => {
		if ($page.url.searchParams.has('connected')) {
			if ($page.url.searchParams.get('connected') === 'spotify') {
				saveMessage = { type: 'success', text: 'Spotify account connected successfully!' };
				setTimeout(() => {
					saveMessage = null;
					// Clear URL params
					goto('/settings', { replaceState: true });
				}, 3000);
			}
		}
		if ($page.url.searchParams.has('spotify_error')) {
			const error = $page.url.searchParams.get('spotify_error');
			const errorMessages: Record<string, string> = {
				missing_code: 'OAuth authorization failed: missing code',
				not_configured:
					'Spotify OAuth is not configured. Please set Client ID and Client Secret first.',
				no_refresh_token: 'Failed to get refresh token. Please try again.',
				token_exchange_failed: 'Failed to exchange authorization code for tokens.',
				callback_failed: 'OAuth callback failed. Please try again.'
			};
			saveMessage = {
				type: 'error',
				text: errorMessages[error || ''] || `Spotify connection failed: ${error}`
			};
			setTimeout(() => {
				saveMessage = null;
				// Clear URL params
				goto('/settings', { replaceState: true });
			}, 5000);
		}
	});
</script>

<div class="min-h-screen bg-gray-50 p-2 sm:p-4 md:p-8">
	<div class="mx-auto max-w-6xl">
		<div class="rounded-lg bg-white shadow-lg">
			<!-- Header -->
			<div
				class="flex flex-col gap-4 border-b border-gray-200 px-4 py-4 sm:flex-row sm:items-center sm:justify-between sm:px-6"
			>
				<div class="flex items-center gap-3">
					<Settings size={24} class="text-gray-700" />
					<h1 class="text-xl font-bold text-gray-900 sm:text-2xl">Settings</h1>
				</div>
				<div class="flex w-full gap-2 sm:w-auto">
					<button
						onclick={handleBackToMirror}
						class="flex cursor-pointer items-center justify-center gap-2 rounded-lg border border-gray-300 bg-white px-4 py-3 text-gray-700 hover:bg-gray-50 sm:py-2"
					>
						<ArrowLeft size={18} />
						<span class="hidden sm:inline">Back to Mirror</span>
						<span class="sm:hidden">Back</span>
					</button>
					<button
						onclick={saveConfig}
						disabled={saving || !config}
						class="flex w-full cursor-pointer items-center justify-center gap-2 rounded-lg px-4 py-3 text-white disabled:cursor-not-allowed disabled:opacity-50 sm:w-auto sm:py-2 {hasUnsavedChanges()
							? 'bg-orange-600 hover:bg-orange-700'
							: 'bg-blue-600 hover:bg-blue-700'}"
					>
						<Save size={18} />
						{saving ? 'Saving...' : 'Save Configuration'}
					</button>
				</div>
			</div>

			{#if saveMessage}
				<div
					class="mx-4 mt-4 flex items-center gap-2 rounded-lg p-4 sm:mx-6 {saveMessage.type ===
					'success'
						? 'bg-green-50 text-green-800'
						: 'bg-red-50 text-red-800'}"
				>
					{#if saveMessage.type === 'success'}
						<CheckCircle2 size={20} />
					{:else}
						<AlertCircle size={20} />
					{/if}
					<span class="text-sm sm:text-base">{saveMessage.text}</span>
				</div>
			{/if}

			{#if loading}
				<div class="p-8 text-center text-gray-500 sm:p-12">Loading configuration...</div>
			{:else if config}
				<div class="flex flex-col lg:flex-row">
					<!-- Mobile Tab Selector -->
					<div class="border-b border-gray-200 bg-gray-50 lg:hidden">
						<select
							bind:value={activeTab}
							class="w-full border-0 bg-transparent px-4 py-3 text-base font-medium text-gray-700 focus:ring-0 focus:outline-none"
						>
							{#each tabs as tab}
								<option value={tab.id}>{tab.label}</option>
							{/each}
						</select>
					</div>

					<!-- Sidebar Tabs (Desktop) -->
					<div class="hidden w-64 border-r border-gray-200 bg-gray-50 lg:block">
						<nav class="space-y-1 p-4">
							{#each tabs as tab}
								<button
									onclick={() => (activeTab = tab.id)}
									class="w-full cursor-pointer rounded-lg px-4 py-2 text-left transition-colors {activeTab ===
									tab.id
										? 'bg-blue-100 font-medium text-blue-700'
										: 'text-gray-700 hover:bg-gray-100'}"
								>
									{tab.label}
								</button>
							{/each}
						</nav>
					</div>

					<!-- Tab Content -->
					<div class="flex-1 p-4 sm:p-6">
						<!-- Dashboard Layout Tab -->
						{#if activeTab === 'dashboard'}
							{@const areaGroups = getComponentsByArea()}
							<div class="space-y-4 sm:space-y-6">
								<h2 class="text-lg font-semibold text-gray-900 sm:text-xl">Dashboard Layout</h2>
								<p class="text-sm text-gray-600 sm:text-base">
									Configure which components are enabled and where they appear on your dashboard.
									Use the arrow buttons to reorder components within each location, or use the
									dropdown to move components between locations.
								</p>

								<div class="space-y-6">
									{#each areaOrder as area}
										{@const componentsInArea = areaGroups[area] || []}
										<div class="space-y-3">
											<h3
												class="border-b border-gray-200 pb-2 text-base font-semibold text-gray-800"
											>
												{getAreaLabel(area)}
											</h3>

											<div class="space-y-2">
												{#if componentsInArea.length === 0}
													<div
														class="rounded-lg border-2 border-dashed border-gray-200 bg-gray-50 py-6 text-center text-sm text-gray-400"
													>
														No components in this area
													</div>
												{:else}
													{#each componentsInArea as { comp, index }, localIndex}
														{@const globalIndex = index}
														<div
															class="flex flex-col gap-3 rounded-lg border border-gray-200 bg-white p-4 transition-colors hover:border-gray-300 sm:flex-row sm:items-center sm:justify-between"
														>
															<div class="flex items-center gap-3 sm:gap-4">
																<!-- Up/Down arrows -->
																<div class="flex flex-col">
																	<button
																		type="button"
																		onclick={() => moveComponentUp(globalIndex)}
																		disabled={!canMoveUp(globalIndex)}
																		class="p-0.5 transition-colors {canMoveUp(globalIndex)
																			? 'cursor-pointer text-gray-600 hover:text-blue-600'
																			: 'cursor-not-allowed text-gray-300'}"
																		aria-label="Move up"
																	>
																		<ChevronUp size={16} />
																	</button>
																	<button
																		type="button"
																		onclick={() => moveComponentDown(globalIndex)}
																		disabled={!canMoveDown(globalIndex)}
																		class="p-0.5 transition-colors {canMoveDown(globalIndex)
																			? 'cursor-pointer text-gray-600 hover:text-blue-600'
																			: 'cursor-not-allowed text-gray-300'}"
																		aria-label="Move down"
																	>
																		<ChevronDown size={16} />
																	</button>
																</div>

																<input
																	type="checkbox"
																	bind:checked={comp.enabled}
																	class="h-5 w-5 shrink-0 cursor-pointer"
																/>
																<span class="font-medium text-gray-900">{comp.id}</span>
															</div>

															<div class="flex items-center gap-4">
																<select
																	bind:value={comp.area}
																	onchange={() => handleAreaChange(globalIndex, comp.area)}
																	class="w-full cursor-pointer rounded border border-gray-300 px-3 py-2 text-sm sm:w-auto sm:py-1"
																>
																	{#each areaOrder as areaOption}
																		<option value={areaOption}>{getAreaLabel(areaOption)}</option>
																	{/each}
																</select>
															</div>
														</div>
													{/each}
												{/if}
											</div>
										</div>
									{/each}
								</div>
							</div>
						{/if}

						<!-- Dynamic Component Tabs -->
						{#each componentManifests as manifest (manifest.id)}
							{#if activeTab === manifest.id}
								<div class="space-y-4 sm:space-y-6">
									<h2 class="text-lg font-semibold text-gray-900 sm:text-xl">
										{manifest.config.title || manifest.name}
									</h2>
									<p class="text-sm text-gray-600 sm:text-base">{manifest.config.description}</p>

									{#if manifest.id in configComponentMap}
										<!-- Custom config component for this component -->
										{@const ConfigComponent = configComponentMap[manifest.id] as any}
										{#if ConfigComponent}
											<ConfigComponent
												{config}
												updateComponentConfig={(key: string, value: any) =>
													updateComponentConfig(manifest.id, key, value)}
											/>
										{/if}
									{/if}

									<div class="space-y-4">
										{#each manifest.config.fields as field}
											{@const componentConfig = getComponentConfig(manifest.id)}
											{@const fieldValue = componentConfig[field.key] ?? field.default}
											{@const hasCustomConfig = manifest.id in configComponentMap}
											{@const shouldHide = hasCustomConfig && field.hiddenWithCustomConfig}
											{#if !shouldHide}
												<ComponentField
													{field}
													value={fieldValue}
													onValueChange={(key, value) =>
														updateComponentConfig(manifest.id, key, value)}
													{passwordVisibility}
													{togglePasswordVisibility}
												/>
											{/if}
										{/each}
									</div>
								</div>
							{/if}
						{/each}
					</div>
				</div>
			{:else}
				<div class="p-8 text-center text-gray-500 sm:p-12">Failed to load configuration</div>
			{/if}
		</div>
	</div>
</div>
