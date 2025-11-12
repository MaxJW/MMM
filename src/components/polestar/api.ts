import type { PolestarConfig, PolestarVehicleData, CarInformation, CarTelematics } from './types';
import { TIMING_STRATEGIES } from '$lib/core/timing';
import { getCached, setCache } from '$lib/core/utils';
import type { CacheEntry } from '$lib/core/utils';

// GraphQL endpoint for Polestar API
const POLESTAR_API_URL = 'https://polestarid.eu.polestar.com/api';
const POLESTAR_GRAPHQL_URL = 'https://polestarid.eu.polestar.com/graphql';

let authToken: string | null = null;
let tokenExpiry: number = 0;
let vehicleDataCache: CacheEntry<PolestarVehicleData> | null = null;
let carInformationCache: CacheEntry<CarInformation> | null = null;
let latestStatusCodeData: number | null = null;
let latestStatusCodeAuth: number | null = null;

interface AuthResponse {
	access_token: string;
	expires_in: number;
	token_type: string;
}

interface GraphQLResponse<T> {
	data?: T;
	errors?: Array<{ message: string }>;
}

// Interface for future use when implementing getAvailableVins
// interface Vehicle {
// 	vin: string;
// 	modelName?: string;
// 	internalVehicleIdentifier?: string;
// }

// interface VehiclesResponse {
// 	vehicles?: Vehicle[];
// }

/**
 * Authenticate with Polestar API using username and password
 */
async function authenticate(username: string, password: string): Promise<string> {
	if (authToken && Date.now() < tokenExpiry) {
		return authToken;
	}

	try {
		const response = await fetch(`${POLESTAR_API_URL}/auth/token`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/x-www-form-urlencoded'
			},
			body: new URLSearchParams({
				username,
				password,
				grant_type: 'password'
			})
		});

		latestStatusCodeAuth = response.status;

		if (!response.ok) {
			throw new Error(`Authentication failed: ${response.status}`);
		}

		const data: AuthResponse = await response.json();
		authToken = data.access_token;
		tokenExpiry = Date.now() + (data.expires_in - 60) * 1000; // Refresh 60s before expiry

		return authToken;
	} catch (error) {
		console.error('Polestar authentication error:', error);
		authToken = null;
		tokenExpiry = 0;
		throw error;
	}
}

/**
 * Execute a GraphQL query against the Polestar API
 */
async function graphqlQuery<T>(
	token: string,
	query: string,
	variables?: Record<string, unknown>
): Promise<T> {
	const response = await fetch(POLESTAR_GRAPHQL_URL, {
		method: 'POST',
		headers: {
			Authorization: `Bearer ${token}`,
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({
			query,
			variables
		})
	});

	latestStatusCodeData = response.status;

	if (!response.ok) {
		throw new Error(`GraphQL request failed: ${response.status}`);
	}

	const result: GraphQLResponse<T> = await response.json();

	if (result.errors && result.errors.length > 0) {
		throw new Error(`GraphQL errors: ${result.errors.map((e) => e.message).join(', ')}`);
	}

	if (!result.data) {
		throw new Error('No data returned from GraphQL query');
	}

	return result.data;
}

/**
 * Get available VINs for the authenticated user
 * (Currently unused but kept for future functionality)
 */
// async function getAvailableVins(token: string): Promise<string[]> {
// 	const query = `
// 		query {
// 			vehicles {
// 				vin
// 			}
// 		}
// 	`;

// 	const data = await graphqlQuery<VehiclesResponse>(token, query);
// 	return data.vehicles?.map((v) => v.vin) || [];
// }

/**
 * Get car information for a specific VIN
 */
async function getCarInformation(token: string, vin: string): Promise<CarInformation> {
	const query = `
		query GetCarInformation($vin: String!) {
			vehicle(vin: $vin) {
				vin
				modelName
				internalVehicleIdentifier
				factoryCompleteDate
				registrationNo
				registrationDate
				torqueNm
				batteryInformation {
					capacity
				}
				softwareVersion
				softwareVersionTimestamp
			}
		}
	`;

	const data = await graphqlQuery<{ vehicle: CarInformation }>(token, query, { vin });
	return {
		...data.vehicle,
		_receivedTimestamp: new Date().toISOString()
	};
}

/**
 * Get car telematics (odometer, battery, health) for a specific VIN
 */
async function getCarTelematics(token: string, vin: string): Promise<CarTelematics> {
	const query = `
		query GetCarTelematics($vin: String!) {
			vehicle(vin: $vin) {
				telematics {
					odometer {
						odometerMeters
						tripMeterAutomaticKm
						tripMeterManualKm
						averageSpeedKmPerHour
						eventUpdatedTimestamp
					}
					battery {
						estimatedDistanceToEmptyKm
						batteryChargeLevelPercentage
						estimatedFullChargeRangeKm
						estimatedChargingTimeToFullMinutes
						chargingStatus
						chargingPowerWatts
						chargingCurrentAmps
						chargerConnectionStatus
						averageEnergyConsumptionKwhPer100km
						estimatedChargingTimeMinutesToTargetDistance
						estimatedFullyCharged
						eventUpdatedTimestamp
					}
					health {
						daysToService
						distanceToServiceKm
						brakeFluidLevelWarning
						engineCoolantLevelWarning
						oilLevelWarning
						serviceWarning
						eventUpdatedTimestamp
					}
				}
			}
		}
	`;

	const data = await graphqlQuery<{ vehicle: { telematics: CarTelematics } }>(token, query, {
		vin
	});
	return data.vehicle.telematics;
}

/**
 * Update latest data for a VIN (triggers a refresh on Polestar's side)
 */
async function updateLatestData(token: string, vin: string): Promise<void> {
	const mutation = `
		mutation UpdateLatestData($vin: String!) {
			updateVehicleData(vin: $vin) {
				success
			}
		}
	`;

	await graphqlQuery(token, mutation, { vin });
}

/**
 * Check if car information needs refresh (should refresh every hour)
 */
function needCarInformationRefresh(carInformation: CarInformation | null): boolean {
	if (!carInformation || !carInformation._receivedTimestamp) {
		return true;
	}

	const lastUpdate = new Date(carInformation._receivedTimestamp);
	const now = new Date();
	const diffMs = now.getTime() - lastUpdate.getTime();
	const oneHour = 60 * 60 * 1000;

	return diffMs > oneHour;
}

/**
 * Main API handler - fetches vehicle data
 */
export async function GET(
	config: PolestarConfig
): Promise<PolestarVehicleData | { error: string }> {
	try {
		const { username, password, vin } = config;

		if (!username || !password) {
			throw new Error('Missing username or password');
		}

		if (!vin) {
			throw new Error('Missing VIN');
		}

		// Authenticate
		const token = await authenticate(username, password);

		// Check cache for vehicle data (refresh every minute)
		const cached = getCached(vehicleDataCache);
		if (cached && cached.vin === vin) {
			return cached;
		}

		// Update latest data first
		try {
			await updateLatestData(token, vin);
		} catch (error) {
			console.warn('Failed to update latest data:', error);
			// Continue anyway, we can still fetch cached data
		}

		// Get car information (cached for 1 hour)
		let carInformation: CarInformation | null = null;
		const cachedInfo = getCached(carInformationCache);
		if (cachedInfo && !needCarInformationRefresh(cachedInfo)) {
			carInformation = cachedInfo;
		} else {
			try {
				carInformation = await getCarInformation(token, vin);
				carInformationCache = setCache(
					carInformationCache,
					carInformation,
					Date.now() + TIMING_STRATEGIES.INFREQUENT.interval
				);
			} catch (error) {
				console.warn('Failed to fetch car information:', error);
			}
		}

		// Get telematics data
		let carTelematics: CarTelematics | null = null;
		try {
			carTelematics = await getCarTelematics(token, vin);
		} catch (error) {
			console.warn('Failed to fetch car telematics:', error);
		}

		// Build response
		const vehicleData: PolestarVehicleData = {
			vin: vin.toUpperCase(),
			carInformation: carInformation || undefined,
			carOdometer: carTelematics?.odometer,
			carBattery: carTelematics?.battery,
			carHealth: carTelematics?.health,
			apiConnected:
				latestStatusCodeData === 200 && latestStatusCodeAuth === 200 && Date.now() < tokenExpiry,
			apiTokenExpiresAt: tokenExpiry ? new Date(tokenExpiry).toISOString() : undefined,
			apiStatusCodeData: latestStatusCodeData || 'Error',
			apiStatusCodeAuth: latestStatusCodeAuth || 'Error'
		};

		// Cache vehicle data
		vehicleDataCache = setCache(
			vehicleDataCache,
			vehicleData,
			Date.now() + TIMING_STRATEGIES.MEDIUM.interval
		);

		return vehicleData;
	} catch (error) {
		console.error('Polestar API error:', error);
		return {
			error: error instanceof Error ? error.message : 'Failed to fetch Polestar vehicle data'
		};
	}
}
