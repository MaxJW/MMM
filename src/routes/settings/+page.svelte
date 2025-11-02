<script lang="ts">
	import { onMount } from 'svelte';
	import type { UserConfig } from '$lib/config/userConfig';
	import Settings from '@lucide/svelte/icons/settings';
	import Save from '@lucide/svelte/icons/save';
	import CheckCircle2 from '@lucide/svelte/icons/check-circle-2';
	import AlertCircle from '@lucide/svelte/icons/alert-circle';
	import Plus from '@lucide/svelte/icons/plus';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import Eye from '@lucide/svelte/icons/eye';
	import EyeOff from '@lucide/svelte/icons/eye-off';
	import GripVertical from '@lucide/svelte/icons/grip-vertical';
	import type { EventConfig } from '$lib/config/events';

	let config: UserConfig | null = $state(null);
	let events: Array<Omit<EventConfig, 'eventImage' | 'qrCode'>> = $state([]);
	let loading = $state(true);
	let saving = $state(false);
	let saveMessage: { type: 'success' | 'error'; text: string } | null = $state(null);
	let activeTab = $state('dashboard');
	let draggedIndex: number | null = $state(null);
	let hoverIndex: number | null = $state(null);

	// Area order for grouping components
	const areaOrder: Array<string> = [
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

	function getAreaOrder(area: string): number {
		const index = areaOrder.indexOf(area);
		return index === -1 ? 999 : index;
	}

	function reorganizeByArea(changedIndex?: number) {
		if (!config) return;

		const components = config.dashboard.components;

		// Create a map of components grouped by area, preserving order within each area
		type ComponentWithMeta = {
			comp: (typeof components)[0];
			originalIndex: number;
			isChanged: boolean;
		};
		const areaGroups: Record<string, ComponentWithMeta[]> = {};

		components.forEach((comp, index) => {
			if (!areaGroups[comp.area]) {
				areaGroups[comp.area] = [];
			}
			areaGroups[comp.area].push({ comp, originalIndex: index, isChanged: index === changedIndex });
		});

		// For the changed item's area, move it to the end of that group
		if (changedIndex !== undefined) {
			const changedComp = components[changedIndex];
			const changedArea = changedComp.area;
			if (areaGroups[changedArea]) {
				// Remove changed item from its current position in the group
				areaGroups[changedArea] = areaGroups[changedArea].filter(
					(item) => item.originalIndex !== changedIndex
				);
				// Add it at the end
				areaGroups[changedArea].push({
					comp: changedComp,
					originalIndex: changedIndex,
					isChanged: true
				});
			}
		}

		// Rebuild the components array in area order, maintaining order within each area
		const reorganized: typeof components = [];
		areaOrder.forEach((area) => {
			if (areaGroups[area]) {
				areaGroups[area].forEach(({ comp }) => {
					reorganized.push(comp);
				});
			}
		});

		// Update the config with reorganized components
		config.dashboard.components = reorganized;
		// Trigger reactivity
		config = { ...config };
	}

	// Password visibility states
	let passwordVisibility: Record<string, boolean> = $state({});
	function togglePasswordVisibility(fieldId: string) {
		passwordVisibility[fieldId] = !(passwordVisibility[fieldId] ?? false);
	}
	function isPasswordVisible(fieldId: string): boolean {
		return passwordVisibility[fieldId] ?? false;
	}

	const tabs = [
		{ id: 'dashboard', label: 'Dashboard Layout' },
		{ id: 'weather', label: 'Weather' },
		{ id: 'google', label: 'Google Calendar' },
		{ id: 'spotify', label: 'Spotify' },
		{ id: 'adguard', label: 'Adguard' },
		{ id: 'energy', label: 'Energy' },
		{ id: 'rss', label: 'RSS Feeds' },
		{ id: 'bin', label: 'Bin Collections' },
		{ id: 'weather-alerts', label: 'Weather Alerts' },
		{ id: 'wifi', label: 'WiFi' },
		{ id: 'events', label: 'Events' }
	];

	onMount(async () => {
		// Load config and events in parallel
		try {
			const [configResponse, eventsResponse] = await Promise.all([
				fetch('/api/config'),
				fetch('/api/events?all=true')
			]);

			if (configResponse.ok) {
				config = await configResponse.json();
			}

			if (eventsResponse.ok) {
				const data = await eventsResponse.json();
				if (data.events) {
					events = data.events;
				}
			}
		} catch (error) {
			console.error('Failed to load configuration:', error);
		} finally {
			loading = false;
		}
	});

	async function saveConfig() {
		if (!config) return;

		saving = true;
		saveMessage = null;

		try {
			// Save config and events in parallel
			const [configResponse, eventsResponse] = await Promise.all([
				fetch('/api/config', {
					method: 'PUT',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify(config)
				}),
				fetch('/api/events', {
					method: 'PUT',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({ events })
				})
			]);

			if (!configResponse.ok) {
				const data = await configResponse.json();
				saveMessage = { type: 'error', text: data.error || 'Failed to save configuration' };
				saving = false;
				return;
			}

			if (!eventsResponse.ok) {
				const data = await eventsResponse.json();
				saveMessage = {
					type: 'error',
					text: data.error || 'Failed to save events configuration'
				};
				saving = false;
				return;
			}

			saveMessage = { type: 'success', text: 'Configuration saved successfully!' };
			setTimeout(() => {
				saveMessage = null;
			}, 3000);
		} catch (error) {
			saveMessage = { type: 'error', text: 'Failed to save configuration' };
		} finally {
			saving = false;
		}
	}

	function addEvent() {
		events = [
			...events,
			{ start: '01-01', end: '01-01', eventText: '', customGreeting: '', eventSlug: '' }
		];
	}

	function removeEvent(index: number) {
		events = events.filter((_, i) => i !== index);
	}

	function addRSSFeed() {
		if (!config) return;
		addArrayItem('rssFeeds', { url: '', sourceName: '' });
	}

	function removeArrayItem(path: string, index: number) {
		if (!config) return;
		const keys = path.split('.');
		let obj: any = config;
		for (let i = 0; i < keys.length - 1; i++) {
			obj = obj[keys[i]];
		}
		obj.splice(index, 1);
		// Trigger reactivity by reassigning (Svelte needs reference change for arrays)
		config = { ...config };
	}

	function addArrayItem(path: string, item: unknown) {
		if (!config) return;
		const keys = path.split('.');
		let obj: any = config;
		for (let i = 0; i < keys.length - 1; i++) {
			if (!obj[keys[i]]) obj[keys[i]] = [];
			obj = obj[keys[i]];
		}
		obj.push(item);
		// Trigger reactivity
		config = { ...config };
	}

	function handleDragStart(index: number, event: DragEvent) {
		if (!event.dataTransfer) return;
		draggedIndex = index;
		event.dataTransfer.effectAllowed = 'move';
		event.dataTransfer.setData('text/html', String(index));
		if (event.target instanceof HTMLElement) {
			event.target.style.opacity = '0.5';
		}
	}

	function handleDragEnd(event: DragEvent) {
		if (event.target instanceof HTMLElement) {
			event.target.style.opacity = '1';
		}
		draggedIndex = null;
		hoverIndex = null;
	}

	function handleDragOver(index: number, event: DragEvent) {
		event.preventDefault();
		if (!event.dataTransfer || draggedIndex === null) return;
		event.dataTransfer.dropEffect = 'move';

		// Update hover index to show placeholder
		if (draggedIndex !== index && event.currentTarget instanceof HTMLElement) {
			const rect = event.currentTarget.getBoundingClientRect();
			const mouseY = event.clientY;
			const midpoint = rect.top + rect.height / 2;

			// Determine where to place the item based on mouse position
			if (draggedIndex < index) {
				// Dragging forward - show placeholder below current item if mouse is in lower half
				hoverIndex = mouseY > midpoint ? index + 1 : index;
			} else {
				// Dragging backward - show placeholder above current item if mouse is in upper half
				hoverIndex = mouseY < midpoint ? index : index + 1;
			}
		}
	}

	function handleDragLeave(event: DragEvent) {
		// Only clear hover if we're actually leaving the item (not just moving to a child)
		if (event.currentTarget instanceof HTMLElement && event.relatedTarget instanceof Node) {
			if (!event.currentTarget.contains(event.relatedTarget)) {
				hoverIndex = null;
			}
		}
	}

	function handleDrop(index: number, event: DragEvent) {
		event.preventDefault();
		if (draggedIndex === null || !config) return;

		const items = config.dashboard.components;

		// Use hoverIndex if available (where placeholder is shown), otherwise use the drop index
		let dropIndex = hoverIndex !== null ? hoverIndex : index;

		// If dropping at the same position or invalid, do nothing
		if (draggedIndex === dropIndex || dropIndex < 0 || dropIndex > items.length) {
			hoverIndex = null;
			draggedIndex = null;
			return;
		}

		// Remove the dragged item from its original position
		const draggedItem = items[draggedIndex];
		items.splice(draggedIndex, 1);

		// Adjust drop index if we removed an item before the drop position
		const adjustedIndex = draggedIndex < dropIndex ? dropIndex - 1 : dropIndex;

		// Insert it at the new position
		items.splice(adjustedIndex, 0, draggedItem);

		// Trigger reactivity by reassigning
		config = { ...config };
		draggedIndex = null;
		hoverIndex = null;
	}
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
				<button
					onclick={saveConfig}
					disabled={saving || !config}
					class="flex w-full items-center justify-center gap-2 rounded-lg bg-blue-600 px-4 py-3 text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50 sm:w-auto sm:py-2"
				>
					<Save size={18} />
					{saving ? 'Saving...' : 'Save Configuration'}
				</button>
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
									class="w-full rounded-lg px-4 py-2 text-left transition-colors {activeTab ===
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
							<div class="space-y-4 sm:space-y-6">
								<h2 class="text-lg font-semibold text-gray-900 sm:text-xl">Dashboard Layout</h2>
								<p class="text-sm text-gray-600 sm:text-base">
									Configure which components are enabled and where they appear on your dashboard.
									Components are automatically grouped by location. Drag items to reorder within a
									location.
								</p>
								<div class="space-y-3 sm:space-y-4">
									{#each config.dashboard.components as comp, i}
										<!-- Placeholder shown before the item when hovering over this position -->
										{#if draggedIndex !== null && hoverIndex === i}
											<div
												class="h-16 rounded-lg border-2 border-dashed border-blue-400 bg-blue-50 transition-all"
											></div>
										{/if}

										<div
											role="button"
											tabindex="0"
											draggable="true"
											ondragstart={(e) => handleDragStart(i, e)}
											ondragend={handleDragEnd}
											ondragover={(e) => handleDragOver(i, e)}
											ondragleave={handleDragLeave}
											ondrop={(e) => handleDrop(i, e)}
											class="flex flex-col gap-3 rounded-lg border border-gray-200 p-4 transition-all {draggedIndex ===
											i
												? 'opacity-30'
												: hoverIndex === i + 1 && draggedIndex !== null
													? 'border-blue-300'
													: ''} sm:flex-row sm:items-center sm:justify-between"
										>
											<div class="flex items-center gap-3 sm:gap-4">
												<div
													class="cursor-move text-gray-400 hover:text-gray-600"
													aria-label="Drag handle"
													style="touch-action: none;"
												>
													<GripVertical size={20} />
												</div>
												<input
													type="checkbox"
													bind:checked={comp.enabled}
													class="h-5 w-5 shrink-0"
													onclick={(e) => e.stopPropagation()}
												/>
												<span class="font-medium text-gray-900">{comp.id}</span>
											</div>
											<div class="flex items-center gap-4">
												<select
													bind:value={comp.area}
													onchange={() => reorganizeByArea(i)}
													class="w-full rounded border border-gray-300 px-3 py-2 text-sm sm:w-auto sm:py-1"
													onclick={(e) => e.stopPropagation()}
													onmousedown={(e) => e.stopPropagation()}
												>
													<option value="top-left">Top Left</option>
													<option value="top-center">Top Center</option>
													<option value="top-right">Top Right</option>
													<option value="middle-left">Middle Left</option>
													<option value="middle-right">Middle Right</option>
													<option value="center">Center</option>
													<option value="bottom-left">Bottom Left</option>
													<option value="bottom-center">Bottom Center</option>
													<option value="bottom-right">Bottom Right</option>
													<option value="notifications">Notifications</option>
												</select>
											</div>
										</div>
									{/each}
									<!-- Placeholder at the end of the list -->
									{#if draggedIndex !== null && hoverIndex === config.dashboard.components.length}
										<div
											class="h-16 rounded-lg border-2 border-dashed border-blue-400 bg-blue-50 transition-all"
										></div>
									{/if}
								</div>
							</div>
						{/if}

						<!-- Google Calendar Tab -->
						{#if activeTab === 'google'}
							<div class="space-y-4 sm:space-y-6">
								<h2 class="text-lg font-semibold text-gray-900 sm:text-xl">
									Google Calendar Settings
								</h2>
								<p class="text-sm text-gray-600 sm:text-base">
									Configure OAuth credentials for Google Calendar integration.
								</p>
								<div class="space-y-4">
									<div>
										<label
											for="google-client-id"
											class="mb-1 block text-sm font-medium text-gray-700"
										>
											Client ID
										</label>
										<input
											id="google-client-id"
											type="text"
											bind:value={config.google.clientId}
											class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
											placeholder="Enter Google OAuth Client ID"
										/>
									</div>
									<div>
										<label
											for="google-client-secret"
											class="mb-1 block text-sm font-medium text-gray-700"
										>
											Client Secret
										</label>
										<div class="relative">
											<input
												id="google-client-secret"
												type={isPasswordVisible('google-client-secret') ? 'text' : 'password'}
												bind:value={config.google.clientSecret}
												class="w-full rounded-lg border border-gray-300 px-4 py-3 pr-12 text-base sm:py-2 sm:pr-10"
												placeholder="Enter Google OAuth Client Secret"
											/>
											<button
												type="button"
												onclick={() => togglePasswordVisibility('google-client-secret')}
												class="absolute top-1/2 right-3 -translate-y-1/2 p-2 text-gray-500 hover:text-gray-700 sm:p-0"
											>
												{#if isPasswordVisible('google-client-secret')}
													<EyeOff size={20} />
												{:else}
													<Eye size={20} />
												{/if}
											</button>
										</div>
									</div>
								</div>
							</div>
						{/if}

						<!-- Spotify Tab -->
						{#if activeTab === 'spotify'}
							<div class="space-y-4 sm:space-y-6">
								<h2 class="text-lg font-semibold text-gray-900 sm:text-xl">Spotify Settings</h2>
								<p class="text-sm text-gray-600 sm:text-base">
									Configure OAuth credentials for Spotify integration.
								</p>
								<div class="space-y-4">
									<div>
										<label
											for="spotify-client-id"
											class="mb-1 block text-sm font-medium text-gray-700"
										>
											Client ID
										</label>
										<input
											id="spotify-client-id"
											type="text"
											bind:value={config.spotify.clientId}
											class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
											placeholder="Enter Spotify Client ID"
										/>
									</div>
									<div>
										<label
											for="spotify-client-secret"
											class="mb-1 block text-sm font-medium text-gray-700"
										>
											Client Secret
										</label>
										<div class="relative">
											<input
												id="spotify-client-secret"
												type={isPasswordVisible('spotify-client-secret') ? 'text' : 'password'}
												bind:value={config.spotify.clientSecret}
												class="w-full rounded-lg border border-gray-300 px-4 py-3 pr-12 text-base sm:py-2 sm:pr-10"
												placeholder="Enter Spotify Client Secret"
											/>
											<button
												type="button"
												onclick={() => togglePasswordVisibility('spotify-client-secret')}
												class="absolute top-1/2 right-3 -translate-y-1/2 p-2 text-gray-500 hover:text-gray-700 sm:p-0"
											>
												{#if isPasswordVisible('spotify-client-secret')}
													<EyeOff size={20} />
												{:else}
													<Eye size={20} />
												{/if}
											</button>
										</div>
									</div>
									<div>
										<label
											for="spotify-refresh-token"
											class="mb-1 block text-sm font-medium text-gray-700"
										>
											Refresh Token
										</label>
										<div class="relative">
											<input
												id="spotify-refresh-token"
												type={isPasswordVisible('spotify-refresh-token') ? 'text' : 'password'}
												bind:value={config.spotify.refreshToken}
												class="w-full rounded-lg border border-gray-300 px-4 py-3 pr-12 text-base sm:py-2 sm:pr-10"
												placeholder="Enter Spotify Refresh Token"
											/>
											<button
												type="button"
												onclick={() => togglePasswordVisibility('spotify-refresh-token')}
												class="absolute top-1/2 right-3 -translate-y-1/2 p-2 text-gray-500 hover:text-gray-700 sm:p-0"
											>
												{#if isPasswordVisible('spotify-refresh-token')}
													<EyeOff size={20} />
												{:else}
													<Eye size={20} />
												{/if}
											</button>
										</div>
									</div>
								</div>
							</div>
						{/if}

						<!-- Adguard Tab -->
						{#if activeTab === 'adguard'}
							<div class="space-y-4 sm:space-y-6">
								<h2 class="text-lg font-semibold text-gray-900 sm:text-xl">
									AdGuard Home Settings
								</h2>
								<p class="text-sm text-gray-600 sm:text-base">
									Configure your AdGuard Home instance connection details.
								</p>
								<div class="space-y-4">
									<div>
										<label for="adguard-url" class="mb-1 block text-sm font-medium text-gray-700">
											AdGuard Home URL
										</label>
										<input
											id="adguard-url"
											type="text"
											bind:value={config.adguard.url}
											class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
											placeholder="e.g., http://192.168.1.100:3000"
										/>
									</div>
									<div>
										<label for="adguard-token" class="mb-1 block text-sm font-medium text-gray-700">
											API Token
										</label>
										<div class="relative">
											<input
												id="adguard-token"
												type={isPasswordVisible('adguard-token') ? 'text' : 'password'}
												bind:value={config.adguard.token}
												class="w-full rounded-lg border border-gray-300 px-4 py-3 pr-12 text-base sm:py-2 sm:pr-10"
												placeholder="Enter AdGuard Home API token"
											/>
											<button
												type="button"
												onclick={() => togglePasswordVisibility('adguard-token')}
												class="absolute top-1/2 right-3 -translate-y-1/2 p-2 text-gray-500 hover:text-gray-700 sm:p-0"
											>
												{#if isPasswordVisible('adguard-token')}
													<EyeOff size={20} />
												{:else}
													<Eye size={20} />
												{/if}
											</button>
										</div>
									</div>
								</div>
							</div>
						{/if}

						<!-- Energy Tab -->
						{#if activeTab === 'energy'}
							<div class="space-y-4 sm:space-y-6">
								<h2 class="text-lg font-semibold text-gray-900 sm:text-xl">
									Energy Usage Settings
								</h2>
								<p class="text-sm text-gray-600 sm:text-base">
									Configure your energy usage API credentials.
								</p>
								<div class="space-y-4">
									<div>
										<label
											for="energy-api-key"
											class="mb-1 block text-sm font-medium text-gray-700"
										>
											API Key
										</label>
										<div class="relative">
											<input
												id="energy-api-key"
												type={isPasswordVisible('energy-api-key') ? 'text' : 'password'}
												bind:value={config.energy.apiKey}
												class="w-full rounded-lg border border-gray-300 px-4 py-3 pr-12 text-base sm:py-2 sm:pr-10"
												placeholder="Enter energy API key"
											/>
											<button
												type="button"
												onclick={() => togglePasswordVisibility('energy-api-key')}
												class="absolute top-1/2 right-3 -translate-y-1/2 p-2 text-gray-500 hover:text-gray-700 sm:p-0"
											>
												{#if isPasswordVisible('energy-api-key')}
													<EyeOff size={20} />
												{:else}
													<Eye size={20} />
												{/if}
											</button>
										</div>
									</div>
								</div>
							</div>
						{/if}

						<!-- RSS Feeds Tab -->
						{#if activeTab === 'rss'}
							<div class="space-y-4 sm:space-y-6">
								<h2 class="text-lg font-semibold text-gray-900 sm:text-xl">RSS Feeds Settings</h2>
								<p class="text-sm text-gray-600 sm:text-base">
									Manage your RSS feed sources for the news widget.
								</p>
								<div class="space-y-4">
									<button
										onclick={addRSSFeed}
										class="flex w-full items-center justify-center gap-2 rounded-lg bg-blue-600 px-4 py-3 text-white hover:bg-blue-700 sm:w-auto sm:py-2"
									>
										<Plus size={18} />
										Add RSS Feed
									</button>
									{#each config.rssFeeds as feed, i}
										<div class="flex flex-col gap-3 rounded-lg border border-gray-200 p-4">
											<div class="flex items-center justify-between">
												<span class="text-sm font-medium text-gray-700">Feed {i + 1}</span>
												<button
													onclick={() => removeArrayItem('rssFeeds', i)}
													class="p-2 text-red-600 hover:text-red-700 sm:p-0"
												>
													<Trash2 size={18} />
												</button>
											</div>
											<div>
												<label
													for="rss-url-{i}"
													class="mb-1 block text-sm font-medium text-gray-700"
												>
													Feed URL
												</label>
												<input
													id="rss-url-{i}"
													type="text"
													bind:value={feed.url}
													class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
													placeholder="https://example.com/feed.xml"
												/>
											</div>
											<div>
												<label
													for="rss-name-{i}"
													class="mb-1 block text-sm font-medium text-gray-700"
												>
													Source Name
												</label>
												<input
													id="rss-name-{i}"
													type="text"
													bind:value={feed.sourceName}
													class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
													placeholder="e.g., Sky News"
												/>
											</div>
										</div>
									{/each}
								</div>
							</div>
						{/if}

						<!-- Bin Collections Tab -->
						{#if activeTab === 'bin'}
							<div class="space-y-4 sm:space-y-6">
								<h2 class="text-lg font-semibold text-gray-900 sm:text-xl">
									Bin Collections Settings
								</h2>
								<p class="text-sm text-gray-600 sm:text-base">
									Configure your bin collection service details.
								</p>
								<div class="space-y-4">
									<div>
										<label for="bin-uprn" class="mb-1 block text-sm font-medium text-gray-700">
											UPRN (Unique Property Reference Number)
										</label>
										<input
											id="bin-uprn"
											type="text"
											bind:value={config.binCollections.uprn}
											class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
											placeholder="Enter your UPRN"
										/>
									</div>
									<div>
										<label
											for="bin-api-endpoint"
											class="mb-1 block text-sm font-medium text-gray-700"
										>
											API Endpoint
										</label>
										<input
											id="bin-api-endpoint"
											type="text"
											bind:value={config.binCollections.apiEndpoint}
											class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
											placeholder="e.g., https://api.example.com/bin-collections"
										/>
									</div>
									<div>
										<label for="bin-council" class="mb-1 block text-sm font-medium text-gray-700">
											Council
										</label>
										<input
											id="bin-council"
											type="text"
											bind:value={config.binCollections.council}
											class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
											placeholder="e.g., Fife Council"
										/>
									</div>
								</div>
							</div>
						{/if}

						<!-- Weather Alerts Tab -->
						{#if activeTab === 'weather-alerts'}
							<div class="space-y-4 sm:space-y-6">
								<h2 class="text-lg font-semibold text-gray-900 sm:text-xl">
									Weather Alerts Settings
								</h2>
								<p class="text-sm text-gray-600 sm:text-base">
									Configure weather alert region settings.
								</p>
								<div class="space-y-4">
									<div>
										<label
											for="weather-alerts-country"
											class="mb-1 block text-sm font-medium text-gray-700"
										>
											Country
										</label>
										<input
											id="weather-alerts-country"
											type="text"
											bind:value={config.weatherAlerts.country}
											class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
											placeholder="e.g., united-kingdom"
										/>
									</div>
									<div>
										<label
											for="weather-alerts-province"
											class="mb-1 block text-sm font-medium text-gray-700"
										>
											Province/Region
										</label>
										<input
											id="weather-alerts-province"
											type="text"
											bind:value={config.weatherAlerts.province}
											class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
											placeholder="e.g., Central, Tayside & Fife"
										/>
									</div>
								</div>
							</div>
						{/if}

						<!-- Weather Tab -->
						{#if activeTab === 'weather'}
							<div class="space-y-4 sm:space-y-6">
								<h2 class="text-lg font-semibold text-gray-900 sm:text-xl">Weather Settings</h2>
								<div class="space-y-4">
									<div>
										<label
											for="weather-api-key"
											class="mb-1 block text-sm font-medium text-gray-700"
										>
											Pirate Weather API Key
										</label>
										<div class="relative">
											<input
												id="weather-api-key"
												type={isPasswordVisible('weather-api-key') ? 'text' : 'password'}
												bind:value={config.weather.apiKey}
												class="w-full rounded-lg border border-gray-300 px-4 py-3 pr-12 text-base sm:py-2 sm:pr-10"
												placeholder="Enter API key"
											/>
											<button
												type="button"
												onclick={() => togglePasswordVisibility('weather-api-key')}
												class="absolute top-1/2 right-3 -translate-y-1/2 p-2 text-gray-500 hover:text-gray-700 sm:p-0"
											>
												{#if isPasswordVisible('weather-api-key')}
													<EyeOff size={20} />
												{:else}
													<Eye size={20} />
												{/if}
											</button>
										</div>
									</div>
									<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
										<div>
											<label
												for="weather-latitude"
												class="mb-1 block text-sm font-medium text-gray-700"
											>
												Latitude
											</label>
											<input
												id="weather-latitude"
												type="text"
												bind:value={config.weather.latitude}
												class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
												placeholder="e.g., 56.0"
											/>
										</div>
										<div>
											<label
												for="weather-longitude"
												class="mb-1 block text-sm font-medium text-gray-700"
											>
												Longitude
											</label>
											<input
												id="weather-longitude"
												type="text"
												bind:value={config.weather.longitude}
												class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
												placeholder="e.g., -3.2"
											/>
										</div>
									</div>
								</div>
							</div>
						{/if}

						{#if activeTab === 'wifi'}
							<div class="space-y-4 sm:space-y-6">
								<h2 class="text-lg font-semibold text-gray-900 sm:text-xl">WiFi Settings</h2>
								<p class="text-sm text-gray-600 sm:text-base">
									Configure your WiFi network details. A QR code will be generated automatically.
								</p>
								<div class="space-y-4">
									<div>
										<label
											for="wifi-network-name"
											class="mb-1 block text-sm font-medium text-gray-700"
										>
											Network Name (SSID)
										</label>
										<input
											id="wifi-network-name"
											type="text"
											bind:value={config.wifi.networkName}
											class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
											placeholder="Enter network name"
										/>
									</div>
									<div>
										<label for="wifi-password" class="mb-1 block text-sm font-medium text-gray-700"
											>Password</label
										>
										<div class="relative">
											<input
												id="wifi-password"
												type={isPasswordVisible('wifi-password') ? 'text' : 'password'}
												bind:value={config.wifi.password}
												class="w-full rounded-lg border border-gray-300 px-4 py-3 pr-12 text-base sm:py-2 sm:pr-10"
												placeholder="Enter WiFi password"
											/>
											<button
												type="button"
												onclick={() => togglePasswordVisibility('wifi-password')}
												class="absolute top-1/2 right-3 -translate-y-1/2 p-2 text-gray-500 hover:text-gray-700 sm:p-0"
											>
												{#if isPasswordVisible('wifi-password')}
													<EyeOff size={20} />
												{:else}
													<Eye size={20} />
												{/if}
											</button>
										</div>
									</div>
									<div>
										<label
											for="wifi-security-type"
											class="mb-1 block text-sm font-medium text-gray-700"
										>
											Security Type
										</label>
										<select
											id="wifi-security-type"
											bind:value={config.wifi.securityType}
											class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
										>
											<option value="WPA">WPA/WPA2</option>
											<option value="WEP">WEP</option>
											<option value="nopass">No Password</option>
										</select>
									</div>
								</div>
							</div>
						{/if}

						<!-- Events Tab -->
						{#if activeTab === 'events'}
							<div class="space-y-4 sm:space-y-6">
								<h2 class="text-lg font-semibold text-gray-900 sm:text-xl">Events Settings</h2>
								<p class="text-sm text-gray-600 sm:text-base">
									Configure recurring events with custom greetings and images. Date format: MM-DD
									(e.g., 10-31 for October 31st).
								</p>
								<div class="space-y-4">
									<button
										onclick={addEvent}
										class="flex w-full items-center justify-center gap-2 rounded-lg bg-blue-600 px-4 py-3 text-white hover:bg-blue-700 sm:w-auto sm:py-2"
									>
										<Plus size={18} />
										Add Event
									</button>
									{#each events as event, i}
										<div class="flex flex-col gap-3 rounded-lg border border-gray-200 p-4">
											<div class="flex items-center justify-between">
												<span class="text-sm font-medium text-gray-700">Event {i + 1}</span>
												<button
													onclick={() => removeEvent(i)}
													class="p-2 text-red-600 hover:text-red-700 sm:p-0"
												>
													<Trash2 size={18} />
												</button>
											</div>
											<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
												<div>
													<label
														for="event-start-{i}"
														class="mb-1 block text-sm font-medium text-gray-700"
													>
														Start Date (MM-DD)
													</label>
													<input
														id="event-start-{i}"
														type="text"
														bind:value={event.start}
														class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
														placeholder="10-31"
														pattern="\d{2}-\d{2}"
													/>
												</div>
												<div>
													<label
														for="event-end-{i}"
														class="mb-1 block text-sm font-medium text-gray-700"
													>
														End Date (MM-DD)
													</label>
													<input
														id="event-end-{i}"
														type="text"
														bind:value={event.end}
														class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
														placeholder="11-01"
														pattern="\d{2}-\d{2}"
													/>
												</div>
											</div>
											<div>
												<label
													for="event-slug-{i}"
													class="mb-1 block text-sm font-medium text-gray-700"
												>
													Event Slug (Folder name in assets/events/)
												</label>
												<input
													id="event-slug-{i}"
													type="text"
													bind:value={event.eventSlug}
													class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
													placeholder="e.g., halloween"
												/>
											</div>
											<div>
												<label
													for="event-text-{i}"
													class="mb-1 block text-sm font-medium text-gray-700"
												>
													Event Text
												</label>
												<input
													id="event-text-{i}"
													type="text"
													bind:value={event.eventText}
													class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
													placeholder="Optional event description"
												/>
											</div>
											<div>
												<label
													for="event-greeting-{i}"
													class="mb-1 block text-sm font-medium text-gray-700"
												>
													Custom Greeting
												</label>
												<input
													id="event-greeting-{i}"
													type="text"
													bind:value={event.customGreeting}
													class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
													placeholder="e.g., 🎃 Happy Halloween! 🎃"
												/>
											</div>
										</div>
									{/each}
								</div>
							</div>
						{/if}
					</div>
				</div>
			{:else}
				<div class="p-8 text-center text-gray-500 sm:p-12">Failed to load configuration</div>
			{/if}
		</div>
	</div>
</div>
