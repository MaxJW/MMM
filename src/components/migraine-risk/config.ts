/** Settings from manifest / `components.migraine-risk` in config.json */

export interface MigraineRiskComponentConfig {
	showGauge?: boolean;
	showBreakdown?: boolean;
	onlyContributingFactors?: boolean;
	showTomorrow?: boolean;
	showAirQualityNote?: boolean;
	showMaxScore?: boolean;
	compactLayout?: boolean;
}

export function resolveMigraineRiskUiOptions(
	config: MigraineRiskComponentConfig | undefined
): Required<MigraineRiskComponentConfig> {
	const c = config ?? {};
	return {
		showGauge: c.showGauge !== false,
		showBreakdown: c.showBreakdown !== false,
		onlyContributingFactors: c.onlyContributingFactors === true,
		showTomorrow: c.showTomorrow !== false,
		showAirQualityNote: c.showAirQualityNote !== false,
		showMaxScore: c.showMaxScore !== false,
		compactLayout: c.compactLayout === true
	};
}
