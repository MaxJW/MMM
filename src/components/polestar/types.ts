export interface PolestarConfig {
	username?: string;
	password?: string;
	vin?: string;
}

export interface CarInformation {
	vin: string;
	modelName: string;
	internalVehicleIdentifier?: string;
	factoryCompleteDate?: string;
	registrationNo?: string;
	registrationDate?: string;
	torqueNm?: number;
	batteryInformation?: {
		capacity: number;
	};
	softwareVersion?: string;
	softwareVersionTimestamp?: string;
	_receivedTimestamp?: string;
}

export interface CarOdometer {
	odometerMeters?: number;
	tripMeterAutomaticKm?: number;
	tripMeterManualKm?: number;
	averageSpeedKmPerHour?: number;
	eventUpdatedTimestamp?: string;
}

export interface CarBattery {
	estimatedDistanceToEmptyKm?: number;
	batteryChargeLevelPercentage?: number;
	estimatedFullChargeRangeKm?: number;
	estimatedChargingTimeToFullMinutes?: number;
	chargingStatus?: string;
	chargingPowerWatts?: number;
	chargingCurrentAmps?: number;
	chargerConnectionStatus?: string;
	averageEnergyConsumptionKwhPer100km?: number;
	estimatedChargingTimeMinutesToTargetDistance?: number;
	estimatedFullyCharged?: string;
	eventUpdatedTimestamp?: string;
}

export interface CarHealth {
	daysToService?: number;
	distanceToServiceKm?: number;
	brakeFluidLevelWarning?: boolean;
	engineCoolantLevelWarning?: boolean;
	oilLevelWarning?: boolean;
	serviceWarning?: boolean;
	eventUpdatedTimestamp?: string;
}

export interface CarTelematics {
	odometer: CarOdometer;
	battery: CarBattery;
	health: CarHealth;
}

export interface PolestarVehicleData {
	vin: string;
	carInformation?: CarInformation;
	carOdometer?: CarOdometer;
	carBattery?: CarBattery;
	carHealth?: CarHealth;
	apiConnected?: boolean;
	apiTokenExpiresAt?: string;
	apiStatusCodeData?: number | string;
	apiStatusCodeAuth?: number | string;
}
