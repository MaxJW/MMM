import { json } from '@sveltejs/kit';
import type { RequestHandler } from '@sveltejs/kit';
import QRCode from 'qrcode';
import { getWiFiConfig } from '$lib/config/userConfig';

export const GET: RequestHandler = async () => {
	try {
		const config = await getWiFiConfig();

		if (!config.networkName) {
			return json({ error: 'WiFi network name not configured' }, { status: 400 });
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

		// Generate QR code as PNG buffer (800x800)
		// White QR code on transparent background
		const qrCodeBuffer = await QRCode.toBuffer(wifiString, {
			width: 800,
			margin: 2,
			color: {
				dark: '#FFFFFF', // White QR code pattern
				light: '#00000000' // Transparent background (RGBA with alpha 0)
			}
		});

		// Convert buffer to base64 data URL
		const base64 = qrCodeBuffer.toString('base64');
		const dataUrl = `data:image/png;base64,${base64}`;

		return json({ qrCode: dataUrl, networkName: config.networkName });
	} catch (error) {
		console.error('Error generating WiFi QR code:', error);
		return json({ error: 'Failed to generate QR code' }, { status: 500 });
	}
};
