<script lang="ts">
	import Eye from '@lucide/svelte/icons/eye';
	import EyeOff from '@lucide/svelte/icons/eye-off';
	import Plus from '@lucide/svelte/icons/plus';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import type { ComponentConfigField } from './types';

	export let field: ComponentConfigField;
	export let value: any;
	export let onValueChange: (key: string, value: any) => void;
	export let passwordVisibility: Record<string, boolean> = {};
	export let togglePasswordVisibility: (key: string) => void = () => {};

	const fieldId = `field-${field.key}`;
	const isPassword = field.type === 'password';
	const showPassword = passwordVisibility[fieldId] ?? false;

	// Color options for calendar colorClass field
	const colorOptions = [
		{ value: 'gray-400', label: 'Gray 400', bgClass: 'bg-gray-400' },
		{ value: 'gray-500', label: 'Gray 500', bgClass: 'bg-gray-500' },
		{ value: 'green-500', label: 'Green 500', bgClass: 'bg-green-500' },
		{ value: 'green-600', label: 'Green 600', bgClass: 'bg-green-600' },
		{ value: 'blue-500', label: 'Blue 500', bgClass: 'bg-blue-500' },
		{ value: 'blue-600', label: 'Blue 600', bgClass: 'bg-blue-600' },
		{ value: 'purple-500', label: 'Purple 500', bgClass: 'bg-purple-500' },
		{ value: 'purple-600', label: 'Purple 600', bgClass: 'bg-purple-600' },
		{ value: 'red-500', label: 'Red 500', bgClass: 'bg-red-500' },
		{ value: 'red-600', label: 'Red 600', bgClass: 'bg-red-600' },
		{ value: 'yellow-500', label: 'Yellow 500', bgClass: 'bg-yellow-500' },
		{ value: 'yellow-600', label: 'Yellow 600', bgClass: 'bg-yellow-600' },
		{ value: 'orange-500', label: 'Orange 500', bgClass: 'bg-orange-500' },
		{ value: 'orange-600', label: 'Orange 600', bgClass: 'bg-orange-600' },
		{ value: 'pink-500', label: 'Pink 500', bgClass: 'bg-pink-500' },
		{ value: 'pink-600', label: 'Pink 600', bgClass: 'bg-pink-600' },
		{ value: 'indigo-500', label: 'Indigo 500', bgClass: 'bg-indigo-500' },
		{ value: 'indigo-600', label: 'Indigo 600', bgClass: 'bg-indigo-600' },
		{ value: 'teal-500', label: 'Teal 500', bgClass: 'bg-teal-500' },
		{ value: 'teal-600', label: 'Teal 600', bgClass: 'bg-teal-600' },
		{ value: 'cyan-500', label: 'Cyan 500', bgClass: 'bg-cyan-500' },
		{ value: 'cyan-600', label: 'Cyan 600', bgClass: 'bg-cyan-600' }
	];

	function handleChange(event: Event) {
		const target = event.target as HTMLInputElement | HTMLSelectElement;
		let newValue: any = target.value;

		if (field.type === 'number') {
			newValue = target.value ? parseFloat(target.value) : undefined;
		}

		onValueChange(field.key, newValue);
	}

	function handleArrayAdd() {
		if (field.type === 'array') {
			const currentArray = Array.isArray(value) ? value : [];
			let newItem: any = {};

			// If itemFields is defined, initialize object with field defaults
			if (field.itemFields) {
				field.itemFields.forEach((f) => {
					newItem[f.key] = f.default ?? '';
				});
			} else {
				// Legacy: if itemSchema is defined, create empty object
				newItem = {};
			}

			onValueChange(field.key, [...currentArray, newItem]);
		}
	}

	function handleArrayItemFieldChange(arrayIndex: number, fieldKey: string, fieldValue: any) {
		if (!Array.isArray(value)) return;
		const newArray = [...value];
		if (!newArray[arrayIndex]) {
			newArray[arrayIndex] = {};
		}
		newArray[arrayIndex][fieldKey] = fieldValue;
		onValueChange(field.key, newArray);
	}

	function handleArrayRemove(index: number) {
		if (field.type === 'array' && Array.isArray(value)) {
			const newArray = [...value];
			newArray.splice(index, 1);
			onValueChange(field.key, newArray);
		}
	}
</script>

<div class="space-y-2">
	<label for={fieldId} class="block text-sm font-medium text-gray-700">
		{field.label}
	</label>
	<p class="mb-2 text-xs text-gray-500">{field.description}</p>

	{#if field.type === 'text' || field.type === 'password'}
		<div class="relative">
			<input
				id={fieldId}
				type={isPassword && !showPassword ? 'password' : 'text'}
				value={value ?? field.default ?? ''}
				oninput={handleChange}
				placeholder={field.placeholder}
				class="w-full rounded-lg border border-gray-300 px-4 py-3 pr-12 text-base sm:py-2 {isPassword
					? 'sm:pr-10'
					: ''}"
			/>
			{#if isPassword}
				<button
					type="button"
					onclick={() => togglePasswordVisibility(fieldId)}
					class="absolute top-1/2 right-3 -translate-y-1/2 p-2 text-gray-500 hover:text-gray-700 sm:p-0"
				>
					{#if showPassword}
						<EyeOff size={20} />
					{:else}
						<Eye size={20} />
					{/if}
				</button>
			{/if}
		</div>
	{:else if field.type === 'number'}
		<input
			id={fieldId}
			type="number"
			value={value ?? field.default ?? ''}
			oninput={handleChange}
			placeholder={field.placeholder}
			class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
		/>
	{:else if field.type === 'select'}
		<select
			id={fieldId}
			value={value ?? field.default ?? ''}
			onchange={handleChange}
			class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
		>
			{#if field.options}
				{#each field.options as option}
					<option value={option.value}>{option.label}</option>
				{/each}
			{/if}
		</select>
	{:else if field.type === 'color'}
		<select
			id={fieldId}
			value={value ?? field.default ?? ''}
			onchange={handleChange}
			class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
		>
			{#each colorOptions as color}
				<option value={color.value}>{color.label}</option>
			{/each}
		</select>
		<div class="mt-2 flex flex-wrap gap-2">
			{#each colorOptions as color}
				<button
					type="button"
					onclick={() => onValueChange(field.key, color.value)}
					class="flex items-center gap-2 rounded-lg border-2 px-3 py-2 text-sm transition-all {value ===
					color.value
						? 'border-blue-500 ring-2 ring-blue-200'
						: 'border-gray-200 hover:border-gray-300'}"
					title={color.label}
				>
					<div class="h-5 w-5 rounded {color.bgClass}"></div>
					<span>{color.label}</span>
				</button>
			{/each}
		</div>
	{:else if field.type === 'array'}
		<div class="space-y-3">
			{#if Array.isArray(value)}
				{#each value as item, index}
					<div class="flex flex-col gap-3 rounded-lg border border-gray-200 p-4">
						<div class="flex items-center justify-between">
							<span class="text-sm font-medium text-gray-700">Item {index + 1}</span>
							<button
								type="button"
								onclick={() => handleArrayRemove(index)}
								class="p-2 text-red-600 hover:text-red-700 sm:p-0"
							>
								<Trash2 size={18} />
							</button>
						</div>
						{#if field.itemFields}
							<!-- Render multiple fields for object items -->
							<div class="space-y-3">
								{#each field.itemFields as subField}
									{@const fieldItemId = `${fieldId}-${index}-${subField.key}`}
									<div>
										<label for={fieldItemId} class="mb-1 block text-sm font-medium text-gray-700">
											{subField.label}
										</label>
										{#if subField.description}
											<p class="mb-1 text-xs text-gray-500">{subField.description}</p>
										{/if}
										{#if subField.type === 'color'}
											<select
												id={fieldItemId}
												value={item?.[subField.key] ?? subField.default ?? ''}
												onchange={(e) => {
													handleArrayItemFieldChange(
														index,
														subField.key,
														(e.target as HTMLSelectElement).value
													);
												}}
												class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
											>
												{#each colorOptions as color}
													<option value={color.value}>{color.label}</option>
												{/each}
											</select>
											<div class="mt-2 flex flex-wrap gap-2">
												{#each colorOptions as color}
													<button
														type="button"
														onclick={() => {
															handleArrayItemFieldChange(index, subField.key, color.value);
														}}
														class="flex items-center gap-2 rounded-lg border-2 px-3 py-2 text-sm transition-all {item?.[
															subField.key
														] === color.value
															? 'border-blue-500 ring-2 ring-blue-200'
															: 'border-gray-200 hover:border-gray-300'}"
														title={color.label}
													>
														<div class="h-5 w-5 rounded {color.bgClass}"></div>
														<span>{color.label}</span>
													</button>
												{/each}
											</div>
										{:else}
											<input
												id={fieldItemId}
												type={subField.type === 'password' ? 'password' : 'text'}
												value={item?.[subField.key] ?? subField.default ?? ''}
												oninput={(e) => {
													handleArrayItemFieldChange(
														index,
														subField.key,
														(e.target as HTMLInputElement).value
													);
												}}
												placeholder={subField.placeholder}
												class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
											/>
										{/if}
									</div>
								{/each}
							</div>
						{:else if field.itemSchema}
							<!-- Legacy: single field schema or JSON display -->
							<input
								type="text"
								value={typeof item === 'object' ? JSON.stringify(item) : item}
								oninput={(e) => {
									const newArray = [...value];
									try {
										newArray[index] = JSON.parse((e.target as HTMLInputElement).value);
									} catch {
										newArray[index] = (e.target as HTMLInputElement).value;
									}
									onValueChange(field.key, newArray);
								}}
								class="w-full rounded-lg border border-gray-300 px-4 py-3 text-base sm:py-2"
							/>
						{/if}
					</div>
				{/each}
			{/if}
			<button
				type="button"
				onclick={handleArrayAdd}
				class="flex w-full items-center justify-center gap-2 rounded-lg bg-blue-600 px-4 py-3 text-white hover:bg-blue-700 sm:w-auto sm:py-2"
			>
				<Plus size={18} />
				Add Item
			</button>
		</div>
	{/if}
</div>
