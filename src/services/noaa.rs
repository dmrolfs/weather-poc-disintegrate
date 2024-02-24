pub use errors::NoaaWeatherError;

use crate::model::{
    transpose_result, LocationZoneCode, LocationZoneType, WeatherAlert, WeatherFrame, ZoneForecast,
};
use geojson::{Feature, FeatureCollection, GeoJson};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use rust_decimal_macros::dec;
use std::time;
use url::Url;

pub trait ZoneWeatherApi: Send + Sync {
    async fn zone_observation(
        &self, zone: &LocationZoneCode,
    ) -> Result<WeatherFrame, NoaaWeatherError>;

    async fn zone_forecast(
        &self, zone_type: LocationZoneType, zone: &LocationZoneCode,
    ) -> Result<ZoneForecast, NoaaWeatherError>;
}

pub trait AlertApi: Send + Sync {
    async fn active_alerts(&self) -> Result<Vec<WeatherAlert>, NoaaWeatherError>;
}

#[derive(Debug, Clone)]
pub enum NoaaWeatherServices {
    Noaa(NoaaWeatherApi),
    HappyPath(HappyPathWeatherServices),
}

impl ZoneWeatherApi for NoaaWeatherServices {
    async fn zone_observation(
        &self, zone: &LocationZoneCode,
    ) -> Result<WeatherFrame, NoaaWeatherError> {
        match self {
            Self::Noaa(svc) => svc.zone_observation(zone).await,
            Self::HappyPath(svc) => svc.zone_observation(zone).await,
        }
    }

    async fn zone_forecast(
        &self, zone_type: LocationZoneType, zone: &LocationZoneCode,
    ) -> Result<ZoneForecast, NoaaWeatherError> {
        match self {
            Self::Noaa(svc) => svc.zone_forecast(zone_type, zone).await,
            Self::HappyPath(svc) => svc.zone_forecast(zone_type, zone).await,
        }
    }
}

impl AlertApi for NoaaWeatherServices {
    async fn active_alerts(&self) -> Result<Vec<WeatherAlert>, NoaaWeatherError> {
        match self {
            Self::Noaa(svc) => svc.active_alerts().await,
            Self::HappyPath(svc) => svc.active_alerts().await,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoaaWeatherApi {
    client: ClientWithMiddleware,
    base_url: Url,
}

impl NoaaWeatherApi {
    pub fn new(
        base_url: impl Into<Url>, user_agent: HeaderValue,
    ) -> Result<Self, NoaaWeatherError> {
        let base_url = base_url.into();
        if base_url.cannot_be_a_base() {
            return Err(NoaaWeatherError::NotABaseUrl(base_url));
        }

        let client = Self::make_http_client(user_agent)?;
        Ok(Self { client, base_url })
    }

    fn make_http_client(user_agent: HeaderValue) -> Result<ClientWithMiddleware, NoaaWeatherError> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, user_agent);

        let client = reqwest::Client::builder()
            .pool_idle_timeout(time::Duration::from_secs(60))
            .default_headers(headers)
            .pool_max_idle_per_host(5)
            .build()?;

        let retry_policy = ExponentialBackoff::builder()
            .retry_bounds(
                time::Duration::from_millis(1_000),
                time::Duration::from_secs(300),
            )
            .build_with_max_retries(3);

        Ok(reqwest_middleware::ClientBuilder::new(client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn fetch_geojson(&self, label: &str, url: Url) -> Result<GeoJson, NoaaWeatherError> {
        let response = self.client.get(url.clone()).send().await?;
        log_response(label, &url, &response);

        //todo: consider wrapping file output code into a project feature or config setting with
        // output directory -- to log noaa api responses

        // let status_code = response.status();
        let body = response.text().await?;

        // use std::io::Write;
        // let now = iso8601_timestamp::Timestamp::now_utc()
        //     .duration_since(iso8601_timestamp::Timestamp::UNIX_EPOCH)
        //     .whole_seconds();
        // let filename = format!("geojson-{label}-{}.json", now);
        // let mut output = std::fs::File::create(&filename).unwrap();
        // write!(output, "{}", body).unwrap();
        // warn!(?status_code, %url, "saved {label} geo response body to: {filename}");

        let geojson = body.parse()?;
        Ok(geojson)
    }
}

impl ZoneWeatherApi for NoaaWeatherApi {
    #[instrument(level = "debug", skip(self), err)]
    async fn zone_observation(
        &self, zone: &LocationZoneCode,
    ) -> Result<WeatherFrame, NoaaWeatherError> {
        let mut url = self.base_url.clone();
        debug!(
            "zone.as_ref:{} || zone.clone.as_ref:{}",
            zone.as_ref(),
            zone.clone().as_ref()
        );
        url.path_segments_mut()
            .unwrap()
            .push("zones")
            .push("forecast")
            .push(zone.as_ref())
            .push("observations");

        let geojson = self.fetch_geojson("zone_observation", url).await?;
        let features = FeatureCollection::try_from(geojson)?;
        Ok(features.into())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn zone_forecast(
        &self, zone_type: LocationZoneType, zone: &LocationZoneCode,
    ) -> Result<ZoneForecast, NoaaWeatherError> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .unwrap()
            .push("zones")
            .push(zone_type.into())
            .push(zone.as_ref())
            .push("forecast");

        let geojson = self.fetch_geojson("zone_forecast", url).await?;
        let feature = Feature::try_from(geojson)?;
        Ok(ZoneForecast::try_from(feature)?)
    }
}

impl AlertApi for NoaaWeatherApi {
    #[instrument(level = "debug", skip(self), err)]
    async fn active_alerts(&self) -> Result<Vec<WeatherAlert>, NoaaWeatherError> {
        let mut url = self.base_url.clone();
        url.path_segments_mut().unwrap().push("alerts").push("active");

        let geojson = self.fetch_geojson("active_alerts", url).await?;
        let features = FeatureCollection::try_from(geojson)?;
        let alerts = features.features.into_iter().map(WeatherAlert::try_from);
        transpose_result(alerts).map_err(|error| error.into())
    }
}

mod errors {
    use thiserror::Error;
    use url::Url;

    #[derive(Debug, Error)]
    pub enum NoaaWeatherError {
        #[error("supplied Weather API url is not a base url to query: {0}")]
        NotABaseUrl(Url),

        #[error("Weather API call failed: {0}")]
        HttpRequest(#[from] reqwest::Error),

        #[error("error occurred in HTTP middleware calling Weather API: {0}")]
        HttpMiddleware(#[from] reqwest_middleware::Error),

        #[error("failed to parse Weather API GeoJson response: {0}")]
        GeoJson(#[from] geojson::Error),

        #[error("{0}")]
        Weather(#[from] crate::errors::WeatherError),
    }
}

fn log_response(label: &str, endpoint: &Url, response: &reqwest::Response) {
    const MESSAGE: &str = "response recv from services.gov";
    let status = response.status();
    if status.is_success() || status.is_informational() {
        debug!(?endpoint, ?response, "{label}: {MESSAGE}");
    } else if status.is_client_error() {
        warn!(?endpoint, ?response, "{label}: {MESSAGE}");
    } else {
        warn!(?endpoint, ?response, "{label}: {MESSAGE}");
    }
}

#[derive(Debug, Copy, Clone)]
pub struct HappyPathWeatherServices;

impl ZoneWeatherApi for HappyPathWeatherServices {
    async fn zone_observation(
        &self, _zone: &LocationZoneCode,
    ) -> Result<WeatherFrame, NoaaWeatherError> {
        use crate::model;

        Ok(WeatherFrame {
            timestamp: iso8601_timestamp::Timestamp::now_utc(),
            temperature: Some(model::QuantitativeValue {
                value: dec!(72.0),
                max_value: dec!(80.0),
                min_value: dec!(60.0),
                unit_code: "DegreesF".into(),
                quality_control: model::QualityControl::V,
            }),
            dewpoint: Some(model::QuantitativeValue {
                value: dec!(33.2),
                max_value: dec!(36.3),
                min_value: dec!(26.2),
                unit_code: "DegreesF".into(),
                quality_control: model::QualityControl::C,
            }),
            wind_direction: None,
            wind_speed: None,
            wind_gust: None,
            barometric_pressure: None,
            sea_level_pressure: None,
            visibility: None,
            max_temperature_last_24_hours: None,
            min_temperature_last_24_hours: None,
            precipitation_last_hour: None,
            precipitation_last_3_hours: None,
            precipitation_last_6_hours: None,
            relative_humidity: None,
            wind_chill: None,
            heat_index: None,
        })
    }

    async fn zone_forecast(
        &self, _zone_type: LocationZoneType, zone: &LocationZoneCode,
    ) -> Result<ZoneForecast, NoaaWeatherError> {
        Ok(ZoneForecast {
            zone_code: zone.to_string(),
            updated: chrono::Utc::now(),
            periods: vec![crate::model::ForecastDetail {
                name: "Rest of Day".to_string(),
                forecast: "Mostly cloudy. Highs in the lower to mid 70s. Light wind.".to_string(),
            }],
        })
    }
}

impl AlertApi for HappyPathWeatherServices {
    async fn active_alerts(&self) -> Result<Vec<WeatherAlert>, NoaaWeatherError> {
        use crate::model;
        use chrono::Utc;
        use trim_margin::MarginTrimmable;

        Ok(vec![
            WeatherAlert {
                affected_zones: vec![
                    LocationZoneCode::new("MDC031".to_string())
                ],
                status: model::AlertStatus::Actual,
                message_type: model::AlertMessageType::Alert,
                sent: Utc::now() - chrono::Duration::hours(1),
                effective: Utc::now() - chrono::Duration::minutes(55),
                onset: Some(Utc::now() - chrono::Duration::minutes(30)),
                expires: Utc::now() + chrono::Duration::minutes(55),
                ends: Some(Utc::now() + chrono::Duration::hours(1)),
                category: model::AlertCategory::Met,
                severity: model::AlertSeverity::Severe,
                certainty: model::AlertCertainty::Possible,
                urgency: model::AlertUrgency::Immediate,
                event: "High Wind Watch".to_string(),
                headline: Some("High Wind Watch issued".to_string()),
                description: r##"* WHAT...South winds 30 to 40 mph with gusts up to 50 mph possible.
                    |* WHERE...Portions of southeast Louisiana and southeast and southern Mississippi.
                    |* WHEN...From Tuesday afternoon through late Tuesday night.
                    |* IMPACTS...Damaging winds could blow down trees and power lines.
                    |Widespread power outages are possible.
                    |Travel could be difficult, especially for high profile vehicles.
                    |"##.trim_margin().unwrap(),
                instruction: r##"Monitor the latest forecasts and warnings for updates on this
                    |situation. Fasten loose objects or shelter objects in a safe location prior
                    |to the onset of winds."##.trim_margin(),
                response: model::AlertResponse::Prepare,
            }
        ])
    }
}
