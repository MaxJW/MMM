<script lang="ts">
	import Plus from '@lucide/svelte/icons/plus';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import type { UserConfig } from '$lib/core/config';

	export let config: UserConfig | null;
	export let updateComponentConfig: (key: string, value: any) => void;

	function connectSpotifyAccount() {
		window.location.href = '/api/spotify/auth';
	}

	function removeSpotifyAccount(index: number) {
		if (!config) return;

		const accounts =
			(config.components.spotify?.accounts as Array<{ refreshToken: string; name?: string }>) || [];
		if (index >= 0 && index < accounts.length) {
			accounts.splice(index, 1);
			updateComponentConfig('accounts', accounts);
		}
	}

	function getSpotifyAccounts(): Array<{ refreshToken: string; name?: string }> {
		if (!config) return [];
		return (
			(config.components.spotify?.accounts as Array<{ refreshToken: string; name?: string }>) || []
		);
	}
</script>

<div class="space-y-4">
	<div class="rounded-lg border border-gray-200 bg-gray-50 p-4">
		<div class="mb-3 flex items-center justify-between">
			<div>
				<h3 class="text-sm font-semibold text-gray-900">Connected Accounts</h3>
				<p class="text-xs text-gray-600">
					Connect multiple Spotify accounts to display all currently playing tracks
				</p>
			</div>
			<button
				type="button"
				onclick={connectSpotifyAccount}
				class="flex items-center gap-2 rounded-lg bg-green-600 px-4 py-2 text-sm text-white hover:bg-green-700"
			>
				<Plus size={18} />
				Connect Account
			</button>
		</div>
		{#if (() => {
			const accounts = getSpotifyAccounts();
			return accounts.length === 0;
		})()}
			<div class="rounded-lg border-2 border-dashed border-gray-300 bg-white p-6 text-center">
				<p class="text-sm text-gray-500">No accounts connected</p>
				<p class="mt-1 text-xs text-gray-400">
					Click "Connect Account" to add your first Spotify account
				</p>
			</div>
		{:else}
			<div class="space-y-2">
				{#each getSpotifyAccounts() as account, index}
					<div
						class="flex items-center justify-between rounded-lg border border-gray-200 bg-white p-3"
					>
						<div class="flex-1">
							<p class="text-sm font-medium text-gray-900">
								{account.name || `Account ${index + 1}`}
							</p>
							<p class="text-xs text-gray-500">
								Refresh token: {account.refreshToken.substring(0, 20)}...
							</p>
						</div>
						<button
							type="button"
							onclick={() => removeSpotifyAccount(index)}
							class="p-2 text-red-600 hover:text-red-700"
							title="Remove account"
						>
							<Trash2 size={18} />
						</button>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>
