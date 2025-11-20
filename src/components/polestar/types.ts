export interface TelematicsBattery {
	batteryChargeLevelPercentage: number;
	chargingStatus: string;
	estimatedChargingTimeToFullMinutes: number | null;
	estimatedDistanceToEmptyKm: number;
}

export interface TelematicsOdometer {
	odometerMeters: number;
}

export interface PolestarData {
	battery: TelematicsBattery;
	odometer: TelematicsOdometer;
}

export interface PolestarConfig {
	username?: string;
	password?: string;
	vin?: string;
}
