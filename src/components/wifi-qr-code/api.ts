import { generateQRCode } from '$lib/core/utils';

interface WiFiConfig {
	networkName?: string;
	password?: string;
	securityType?: 'WPA' | 'WEP' | 'nopass';
}

export async function GET(config: WiFiConfig): Promise<{ qrCode?: string; networkName?: string; error?: string }> {
	try {
		if (!config.networkName) {
			return { error: 'WiFi network name not configured' };
		}

		// Generate WiFi connection string
		// Format: WIFI:T:WPA;S:NetworkName;P:Password;;
		const securityType = config.securityType || 'WPA';
		const password = config.password || '';

		let wifiString = `WIFI:T:${securityType};S:${config.networkName};`;
		if (password) {
			wifiString += `P:${password};`;
		}
		wifiString += ';';

		// Generate QR code using shared utility
		const qrCode = await generateQRCode(wifiString);

		if (!qrCode) {
			return { error: 'Failed to generate QR code' };
		}

		return { qrCode, networkName: config.networkName };
	} catch (error) {
		console.error('Error generating WiFi QR code:', error);
		return { error: error instanceof Error ? error.message : 'Failed to generate QR code' };
	}
}
