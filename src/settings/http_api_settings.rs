use serde_with::serde_as;
use settings_loader::common::http::HttpServerSettings;
use std::time::Duration;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HttpApiSettings {
    #[serde(flatten)]
    pub server: HttpServerSettings,

    #[serde(alias = "timeout_secs")]
    #[serde_as(as = "serde_with::DurationSeconds<u64>")]
    pub timeout: Duration,

    pub rate_limit: RateLimitSettings,
}

#[serde_as]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize)]
pub struct RateLimitSettings {
    pub burst_size: u32,

    #[serde(alias = "per_seconds")]
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    pub per_duration: Duration,
}
