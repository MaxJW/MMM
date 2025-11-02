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
	import type { EventConfig } from '$lib/config/events';

	let config: UserConfig | null = $state(null);
	let events: Array<Omit<EventConfig, 'eventImage' | 'qrCode'>> = $state([]);
	let loading = $state(true);
	let saving = $state(false);
	let saveMessage: { type: 'success' | 'error'; text: string } | null = $state(null);
	let activeTab = $state('dashboard');

	// Password visibility states
	let passwordVisibility: Record<string, boolean> = $state({});
	function togglePasswordVisibility(fieldId: string) {
		passwordVisibility[fieldId] = !passwordVisibility[fieldId];
		passwordVisibility = { ...passwordVisibility };
	}
	function isPasswordVisible(fieldId: string): boolean {
		return passwordVisibility[fieldId] || false;
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
		try {
			const response = await fetch('/api/config');
			if (response.ok) {
				config = await response.json();
			}
		} catch (error) {
			console.error('Failed to load config:', error);
		}

		// Load events separately
		try {
			const eventsResponse = await fetch('/api/events?all=true');
			if (eventsResponse.ok) {
				const data = await eventsResponse.json();
				if (data.events) {
					events = data.events;
				}
			}
		} catch (error) {
			console.error('Failed to load events:', error);
		} finally {
			loading = false;
		}
	});

	async function saveConfig() {
		if (!config) return;

		saving = true;
		saveMessage = null;

		try {
			// Save main config
			const response = await fetch('/api/config', {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(config)
			});

			if (!response.ok) {
				const data = await response.json();
				saveMessage = { type: 'error', text: data.error || 'Failed to save configuration' };
				return;
			}

			// Save events configuration
			const eventsResponse = await fetch('/api/events', {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ events })
			});

			if (!eventsResponse.ok) {
				const data = await eventsResponse.json();
				saveMessage = {
					type: 'error',
					text: data.error || 'Failed to save events configuration'
				};
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

	function updateConfig(path: string, value: unknown) {
		if (!config) return;
		const keys = path.split('.');
		let obj: any = config;
		for (let i = 0; i < keys.length - 1; i++) {
			if (!obj[keys[i]]) obj[keys[i]] = {};
			obj = obj[keys[i]];
		}
		obj[keys[keys.length - 1]] = value;
		config = { ...config };
	}

	function updateArrayConfig(path: string, index: number, value: unknown) {
		if (!config) return;
		const keys = path.split('.');
		let obj: any = config;
		for (let i = 0; i < keys.length - 1; i++) {
			if (!obj[keys[i]]) obj[keys[i]] = [];
			obj = obj[keys[i]];
		}
		obj[index] = { ...obj[index], ...(value as object) };
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
		config = { ...config };
	}

	function removeArrayItem(path: string, index: number) {
		if (!config) return;
		const keys = path.split('.');
		let obj: any = config;
		for (let i = 0; i < keys.length - 1; i++) {
			obj = obj[keys[i]];
		}
		obj.splice(index, 1);
		config = { ...config };
	}
</script>

<div class="min-h-screen bg-gray-50 p-8">
	<div class="mx-auto max-w-6xl">
		<div class="rounded-lg bg-white shadow-lg">
			<!-- Header -->
			<div class="flex items-center justify-between border-b border-gray-200 px-6 py-4">
				<div class="flex items-center gap-3">
					<Settings size={24} class="text-gray-700" />
					<h1 class="text-2xl font-bold text-gray-900">Settings</h1>
				</div>
				<button
					onclick={saveConfig}
					disabled={saving || !config}
					class="flex items-center gap-2 rounded-lg bg-blue-600 px-4 py-2 text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50"
				>
					<Save size={18} />
					{saving ? 'Saving...' : 'Save Configuration'}
				</button>
			</div>

			{#if saveMessage}
				<div
					class="mx-6 mt-4 flex items-center gap-2 rounded-lg p-4 {saveMessage.type === 'success'
						? 'bg-green-50 text-green-800'
						: 'bg-red-50 text-red-800'}"
				>
					{#if saveMessage.type === 'success'}
						<CheckCircle2 size={20} />
					{:else}
						<AlertCircle size={20} />
					{/if}
					<span>{saveMessage.text}</span>
				</div>
			{/if}

			{#if loading}
				<div class="p-12 text-center text-gray-500">Loading configuration...</div>
			{:else if config}
				<div class="flex">
					<!-- Sidebar Tabs -->
					<div class="w-64 border-r border-gray-200 bg-gray-50">
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
					<div class="flex-1 p-6">
						<!-- Dashboard Layout Tab -->
						{#if activeTab === 'dashboard'}
							<div class="space-y-6">
								<h2 class="text-xl font-semibold text-gray-900">Dashboard Layout</h2>
								<p class="text-gray-600">
									Configure which components are enabled and where they appear on your dashboard.
								</p>
								<div class="space-y-4">
									{#each config.dashboard.components as comp, i}
										<div
											class="flex items-center justify-between rounded-lg border border-gray-200 p-4"
										>
											<div class="flex items-center gap-4">
												<input type="checkbox" bind:checked={comp.enabled} class="h-5 w-5" />
												<span class="font-medium text-gray-900">{comp.id}</span>
											</div>
											<div class="flex items-center gap-4">
												<select
													bind:value={comp.area}
													class="rounded border border-gray-300 px-3 py-1"
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
								</div>
							</div>
						{/if}

						<!-- Google Calendar Tab -->
						{#if activeTab === 'google'}
							<div class="space-y-6">
								<h2 class="text-xl font-semibold text-gray-900">Google Calendar Settings</h2>
								<p class="text-gray-600">
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
											class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
												class="w-full rounded-lg border border-gray-300 px-4 py-2 pr-10"
												placeholder="Enter Google OAuth Client Secret"
											/>
											<button
												type="button"
												onclick={() => togglePasswordVisibility('google-client-secret')}
												class="absolute top-1/2 right-3 -translate-y-1/2 text-gray-500 hover:text-gray-700"
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
							<div class="space-y-6">
								<h2 class="text-xl font-semibold text-gray-900">Spotify Settings</h2>
								<p class="text-gray-600">Configure OAuth credentials for Spotify integration.</p>
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
											class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
												class="w-full rounded-lg border border-gray-300 px-4 py-2 pr-10"
												placeholder="Enter Spotify Client Secret"
											/>
											<button
												type="button"
												onclick={() => togglePasswordVisibility('spotify-client-secret')}
												class="absolute top-1/2 right-3 -translate-y-1/2 text-gray-500 hover:text-gray-700"
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
												class="w-full rounded-lg border border-gray-300 px-4 py-2 pr-10"
												placeholder="Enter Spotify Refresh Token"
											/>
											<button
												type="button"
												onclick={() => togglePasswordVisibility('spotify-refresh-token')}
												class="absolute top-1/2 right-3 -translate-y-1/2 text-gray-500 hover:text-gray-700"
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
							<div class="space-y-6">
								<h2 class="text-xl font-semibold text-gray-900">AdGuard Home Settings</h2>
								<p class="text-gray-600">
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
											class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
												class="w-full rounded-lg border border-gray-300 px-4 py-2 pr-10"
												placeholder="Enter AdGuard Home API token"
											/>
											<button
												type="button"
												onclick={() => togglePasswordVisibility('adguard-token')}
												class="absolute top-1/2 right-3 -translate-y-1/2 text-gray-500 hover:text-gray-700"
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
							<div class="space-y-6">
								<h2 class="text-xl font-semibold text-gray-900">Energy Usage Settings</h2>
								<p class="text-gray-600">Configure your energy usage API credentials.</p>
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
												class="w-full rounded-lg border border-gray-300 px-4 py-2 pr-10"
												placeholder="Enter energy API key"
											/>
											<button
												type="button"
												onclick={() => togglePasswordVisibility('energy-api-key')}
												class="absolute top-1/2 right-3 -translate-y-1/2 text-gray-500 hover:text-gray-700"
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
							<div class="space-y-6">
								<h2 class="text-xl font-semibold text-gray-900">RSS Feeds Settings</h2>
								<p class="text-gray-600">Manage your RSS feed sources for the news widget.</p>
								<div class="space-y-4">
									<button
										onclick={addRSSFeed}
										class="flex items-center gap-2 rounded-lg bg-blue-600 px-4 py-2 text-white hover:bg-blue-700"
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
													class="text-red-600 hover:text-red-700"
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
													class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
													class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
							<div class="space-y-6">
								<h2 class="text-xl font-semibold text-gray-900">Bin Collections Settings</h2>
								<p class="text-gray-600">Configure your bin collection service details.</p>
								<div class="space-y-4">
									<div>
										<label for="bin-uprn" class="mb-1 block text-sm font-medium text-gray-700">
											UPRN (Unique Property Reference Number)
										</label>
										<input
											id="bin-uprn"
											type="text"
											bind:value={config.binCollections.uprn}
											class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
											class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
											class="w-full rounded-lg border border-gray-300 px-4 py-2"
											placeholder="e.g., Fife Council"
										/>
									</div>
								</div>
							</div>
						{/if}

						<!-- Weather Alerts Tab -->
						{#if activeTab === 'weather-alerts'}
							<div class="space-y-6">
								<h2 class="text-xl font-semibold text-gray-900">Weather Alerts Settings</h2>
								<p class="text-gray-600">Configure weather alert region settings.</p>
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
											class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
											class="w-full rounded-lg border border-gray-300 px-4 py-2"
											placeholder="e.g., Central, Tayside & Fife"
										/>
									</div>
								</div>
							</div>
						{/if}

						<!-- Weather Tab -->
						{#if activeTab === 'weather'}
							<div class="space-y-6">
								<h2 class="text-xl font-semibold text-gray-900">Weather Settings</h2>
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
												class="w-full rounded-lg border border-gray-300 px-4 py-2 pr-10"
												placeholder="Enter API key"
											/>
											<button
												type="button"
												onclick={() => togglePasswordVisibility('weather-api-key')}
												class="absolute top-1/2 right-3 -translate-y-1/2 text-gray-500 hover:text-gray-700"
											>
												{#if isPasswordVisible('weather-api-key')}
													<EyeOff size={20} />
												{:else}
													<Eye size={20} />
												{/if}
											</button>
										</div>
									</div>
									<div class="grid grid-cols-2 gap-4">
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
												class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
												class="w-full rounded-lg border border-gray-300 px-4 py-2"
												placeholder="e.g., -3.2"
											/>
										</div>
									</div>
								</div>
							</div>
						{/if}

						{#if activeTab === 'wifi'}
							<div class="space-y-6">
								<h2 class="text-xl font-semibold text-gray-900">WiFi Settings</h2>
								<p class="text-gray-600">
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
											class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
												class="w-full rounded-lg border border-gray-300 px-4 py-2 pr-10"
												placeholder="Enter WiFi password"
											/>
											<button
												type="button"
												onclick={() => togglePasswordVisibility('wifi-password')}
												class="absolute top-1/2 right-3 -translate-y-1/2 text-gray-500 hover:text-gray-700"
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
											class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
							<div class="space-y-6">
								<h2 class="text-xl font-semibold text-gray-900">Events Settings</h2>
								<p class="text-gray-600">
									Configure recurring events with custom greetings and images. Date format: MM-DD
									(e.g., 10-31 for October 31st).
								</p>
								<div class="space-y-4">
									<button
										onclick={addEvent}
										class="flex items-center gap-2 rounded-lg bg-blue-600 px-4 py-2 text-white hover:bg-blue-700"
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
													class="text-red-600 hover:text-red-700"
												>
													<Trash2 size={18} />
												</button>
											</div>
											<div class="grid grid-cols-2 gap-4">
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
														class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
														class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
													class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
													class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
													class="w-full rounded-lg border border-gray-300 px-4 py-2"
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
				<div class="p-12 text-center text-gray-500">Failed to load configuration</div>
			{/if}
		</div>
	</div>
</div>
