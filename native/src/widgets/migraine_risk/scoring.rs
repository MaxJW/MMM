//! Port of src/components/migraine-risk/scoring.ts (thresholds from the MIT-licensed
//! migraine-risk-card sensor package), with every case of scoring.test.ts as a test.

pub const MAX_SCORE: i32 = 22;

/// `RiskLevelLabel`; the labels themselves are drawn by the Slint side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Low,
    Moderate,
    High,
    VeryHigh,
}

pub fn wind_mps_to_kmh(mps: f64) -> f64 {
    mps * 3.6
}

/// Points from 6 h pressure drop (hPa): YAML thresholds.
pub fn pressure_drop_6h_points(drop_hpa: f64) -> i32 {
    match drop_hpa {
        d if d >= 10.0 => 4,
        d if d >= 8.0 => 3,
        d if d >= 6.0 => 2,
        d if d >= 4.0 => 1,
        _ => 0,
    }
}

/// Points from 24 h pressure drop (hPa).
pub fn pressure_drop_24h_points(drop_hpa: f64) -> i32 {
    match drop_hpa {
        d if d >= 14.0 => 3,
        d if d >= 10.0 => 2,
        d if d >= 6.0 => 1,
        _ => 0,
    }
}

pub fn humidity_factor(h: f64) -> i32 {
    if h > 80.0 {
        2
    } else if h < 30.0 {
        1
    } else {
        0
    }
}

pub fn temperature_factor(t: f64) -> i32 {
    if t > 30.0 || t < 5.0 {
        2
    } else {
        0
    }
}

pub fn temperature_change_factor(delta_c: f64) -> i32 {
    match delta_c {
        d if d >= 8.0 => 2,
        d if d >= 5.0 => 1,
        _ => 0,
    }
}

/// Wind in km/h (HA package uses km/h).
pub fn wind_factor(wind_kmh: f64) -> i32 {
    match wind_kmh {
        w if w >= 50.0 => 2,
        w if w >= 35.0 => 1,
        _ => 0,
    }
}

pub fn uv_factor(uv: f64) -> i32 {
    match uv {
        u if u >= 8.0 => 2,
        u if u >= 6.0 => 1,
        _ => 0,
    }
}

/// WAQI-style AQI + optional PM/O3 nudge; capped at 3.
pub fn air_quality_factor(aqi: f64, pm25: f64, pm10: f64, o3: f64) -> i32 {
    let base = if aqi <= 50.0 {
        0
    } else if aqi <= 100.0 {
        1
    } else if aqi <= 150.0 {
        2
    } else {
        3
    };
    let nudge = i32::from(pm25 >= 25.0) + i32::from(pm10 >= 50.0) + i32::from(o3 >= 120.0);
    (base + nudge).min(3)
}

#[derive(Clone, Debug, Default)]
pub struct ThunderstormInputs {
    /// Lowercased text from weather warnings / CAP alerts.
    pub alerts_text: String,
    /// Lowercased forecast text (e.g. summary).
    pub forecast_text: String,
    /// True if current/hourly icon or condition implies thunder.
    pub has_thunder_condition: bool,
}

pub fn thunderstorm_factor(input: &ThunderstormInputs) -> i32 {
    let warning_thunder = input.alerts_text.contains("thunderstorm");
    let forecast_thunder = input.forecast_text.contains("thunderstorm")
        || input.forecast_text.contains("thunder")
        || input.has_thunder_condition;
    if warning_thunder {
        2
    } else if forecast_thunder {
        1
    } else {
        0
    }
}

pub fn today_risk_level(score: i32) -> Level {
    match score {
        s if s >= 12 => Level::VeryHigh,
        s if s >= 8 => Level::High,
        s if s >= 4 => Level::Moderate,
        _ => Level::Low,
    }
}

pub fn tomorrow_risk_level(score: i32) -> Level {
    match score {
        s if s >= 8 => Level::VeryHigh,
        s if s >= 5 => Level::High,
        s if s >= 3 => Level::Moderate,
        _ => Level::Low,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AirQuality {
    pub aqi: f64,
    pub pm25: f64,
    pub pm10: f64,
    pub o3: f64,
}

#[derive(Clone, Debug, Default)]
pub struct FactorInputs {
    pub pressure_drop_6h: f64,
    pub pressure_drop_24h: f64,
    pub humidity_pct: f64,
    pub temp_c: f64,
    pub temp_change_6h: f64,
    pub wind_kmh: f64,
    pub uv_index: f64,
    pub thunderstorm: ThunderstormInputs,
    pub air_quality: AirQuality,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FactorBreakdown {
    pub pressure_6h: i32,
    pub pressure_24h: i32,
    pub humidity: i32,
    pub temperature: i32,
    pub temperature_change: i32,
    pub wind: i32,
    pub uv: i32,
    pub thunderstorm: i32,
    pub air_quality: i32,
}

impl FactorBreakdown {
    /// `factorLabels` of api.ts with the points, in display order.
    pub fn rows(&self) -> [(&'static str, i32); 9] {
        [
            ("Pressure (6h drop)", self.pressure_6h),
            ("Pressure (24h drop)", self.pressure_24h),
            ("Humidity", self.humidity),
            ("Temperature", self.temperature),
            ("Temp. change (6h)", self.temperature_change),
            ("Wind", self.wind),
            ("UV", self.uv),
            ("Thunderstorm", self.thunderstorm),
            ("Air quality", self.air_quality),
        ]
    }
}

pub fn compute_factor_breakdown(args: &FactorInputs) -> FactorBreakdown {
    let aq = args.air_quality;
    FactorBreakdown {
        pressure_6h: pressure_drop_6h_points(args.pressure_drop_6h),
        pressure_24h: pressure_drop_24h_points(args.pressure_drop_24h),
        humidity: humidity_factor(args.humidity_pct),
        temperature: temperature_factor(args.temp_c),
        temperature_change: temperature_change_factor(args.temp_change_6h),
        wind: wind_factor(args.wind_kmh),
        uv: uv_factor(args.uv_index),
        thunderstorm: thunderstorm_factor(&args.thunderstorm),
        air_quality: air_quality_factor(aq.aqi, aq.pm25, aq.pm10, aq.o3),
    }
}

pub fn sum_factors(f: &FactorBreakdown) -> i32 {
    f.rows().iter().map(|(_, pts)| pts).sum()
}

#[derive(Clone, Debug, Default)]
pub struct TomorrowForecastInput {
    pub tmax: f64,
    pub tmin: f64,
    pub uv: f64,
    pub forecast_text_lower: String,
    pub rain_chance_pct: f64,
}

/// Port of sensor `migraine_risk_forecast_tomorrow` state (YAML ~676–704).
/// Returns None if there is no input (tomorrow's max temp is unavailable).
pub fn tomorrow_forecast_score(input: Option<&TomorrowForecastInput>) -> Option<i32> {
    let input = input?;
    let (tmax, tmin, uv) = (input.tmax, input.tmin, input.uv);
    let fc = input.forecast_text_lower.as_str();

    let temp_pts = if tmax > 30.0 || tmin < 5.0 {
        2
    } else if tmax > 28.0 || tmin < 8.0 {
        1
    } else {
        0
    };
    let temp_range = tmax - tmin;
    let swing_pts = if temp_range >= 15.0 {
        2
    } else if temp_range >= 10.0 {
        1
    } else {
        0
    };
    let uv_pts = uv_factor(uv);

    // The TS has separate "damaging/destructive" and "strong/45..60 to" branches, both 2.
    let strong = [
        "damaging winds",
        "destructive winds",
        "strong winds",
        "45 to",
        "50 to",
        "55 to",
        "60 to",
    ];
    let wind_pts = if strong.iter().any(|p| fc.contains(p)) {
        2
    } else if fc.contains("35 to") || fc.contains("40 to") {
        1
    } else {
        0
    };

    let thunder_pts = if fc.contains("thunderstorm") || fc.contains("thunder") {
        2
    } else {
        0
    };
    let hum_pts = i32::from(input.rain_chance_pct >= 80.0);

    Some(temp_pts + swing_pts + uv_pts + wind_pts + thunder_pts + hum_pts)
}

/// Every case of scoring.test.ts, one test per `describe`/`test`.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wind_mps_to_kmh_converts_si_to_kmh() {
        assert_eq!(wind_mps_to_kmh(10.0), 36.0);
        assert_eq!(wind_mps_to_kmh(0.0), 0.0);
    }

    #[test]
    fn pressure_drop_6h_points_matches_yaml_thresholds() {
        assert_eq!(pressure_drop_6h_points(0.0), 0);
        assert_eq!(pressure_drop_6h_points(3.9), 0);
        assert_eq!(pressure_drop_6h_points(4.0), 1);
        assert_eq!(pressure_drop_6h_points(7.9), 2);
        assert_eq!(pressure_drop_6h_points(8.0), 3);
        assert_eq!(pressure_drop_6h_points(9.9), 3);
        assert_eq!(pressure_drop_6h_points(10.0), 4);
    }

    #[test]
    fn pressure_drop_24h_points_matches_yaml_thresholds() {
        assert_eq!(pressure_drop_24h_points(0.0), 0);
        assert_eq!(pressure_drop_24h_points(5.9), 0);
        assert_eq!(pressure_drop_24h_points(6.0), 1);
        assert_eq!(pressure_drop_24h_points(13.9), 2);
        assert_eq!(pressure_drop_24h_points(14.0), 3);
    }

    #[test]
    fn humidity_factor_edges() {
        assert_eq!(humidity_factor(45.0), 0);
        assert_eq!(humidity_factor(29.9), 1);
        assert_eq!(humidity_factor(80.1), 2);
    }

    #[test]
    fn temperature_factor_extremes() {
        assert_eq!(temperature_factor(20.0), 0);
        assert_eq!(temperature_factor(30.1), 2);
        assert_eq!(temperature_factor(4.9), 2);
    }

    #[test]
    fn temperature_change_factor_bands() {
        assert_eq!(temperature_change_factor(4.0), 0);
        assert_eq!(temperature_change_factor(5.0), 1);
        assert_eq!(temperature_change_factor(8.0), 2);
    }

    #[test]
    fn wind_factor_kmh() {
        assert_eq!(wind_factor(34.0), 0);
        assert_eq!(wind_factor(35.0), 1);
        assert_eq!(wind_factor(49.0), 1);
        assert_eq!(wind_factor(50.0), 2);
    }

    #[test]
    fn uv_factor_bands() {
        assert_eq!(uv_factor(5.0), 0);
        assert_eq!(uv_factor(6.0), 1);
        assert_eq!(uv_factor(8.0), 2);
    }

    #[test]
    fn air_quality_factor_base_and_cap() {
        assert_eq!(air_quality_factor(40.0, 0.0, 0.0, 0.0), 0);
        assert_eq!(air_quality_factor(75.0, 0.0, 0.0, 0.0), 1);
        assert_eq!(air_quality_factor(120.0, 0.0, 0.0, 0.0), 2);
        assert_eq!(air_quality_factor(160.0, 0.0, 0.0, 0.0), 3);
        assert_eq!(air_quality_factor(160.0, 30.0, 60.0, 130.0), 3);
    }

    fn thunder(alerts: &str, forecast: &str, condition: bool) -> i32 {
        thunderstorm_factor(&ThunderstormInputs {
            alerts_text: alerts.into(),
            forecast_text: forecast.into(),
            has_thunder_condition: condition,
        })
    }

    #[test]
    fn thunderstorm_factor_warning_beats_forecast() {
        assert_eq!(thunder("severe thunderstorm warning", "", false), 2);
        assert_eq!(thunder("", "possible thunder later", false), 1);
        assert_eq!(thunder("", "", true), 1);
    }

    #[test]
    fn risk_levels_today() {
        assert_eq!(today_risk_level(0), Level::Low);
        assert_eq!(today_risk_level(4), Level::Moderate);
        assert_eq!(today_risk_level(8), Level::High);
        assert_eq!(today_risk_level(12), Level::VeryHigh);
    }

    #[test]
    fn risk_levels_tomorrow_bands() {
        assert_eq!(tomorrow_risk_level(2), Level::Low);
        assert_eq!(tomorrow_risk_level(3), Level::Moderate);
        assert_eq!(tomorrow_risk_level(5), Level::High);
        assert_eq!(tomorrow_risk_level(8), Level::VeryHigh);
    }

    #[test]
    fn tomorrow_forecast_score_returns_none_when_input_none() {
        assert_eq!(tomorrow_forecast_score(None), None);
    }

    #[test]
    fn tomorrow_forecast_score_combined_scenario() {
        let s = tomorrow_forecast_score(Some(&TomorrowForecastInput {
            tmax: 32.0,
            tmin: 10.0,
            uv: 7.0,
            forecast_text_lower: "strong winds and thunderstorm possible".into(),
            rain_chance_pct: 85.0,
        }));
        assert!(s.unwrap() > 0);
        // Beyond the TS test (which only checks > 0): 2 temp + 2 swing + 1 uv + 2 wind
        // + 2 thunder + 1 humidity.
        assert_eq!(s, Some(10));
    }

    #[test]
    fn compute_factor_breakdown_and_sum_all_zeros() {
        let f = compute_factor_breakdown(&FactorInputs {
            pressure_drop_6h: 0.0,
            pressure_drop_24h: 0.0,
            humidity_pct: 50.0,
            temp_c: 20.0,
            temp_change_6h: 0.0,
            wind_kmh: 10.0,
            uv_index: 0.0,
            thunderstorm: ThunderstormInputs::default(),
            air_quality: AirQuality::default(),
        });
        assert_eq!(sum_factors(&f), 0);
    }
}
