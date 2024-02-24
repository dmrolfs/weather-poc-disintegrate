#[macro_use]
extern crate serde;

#[macro_use]
extern crate tracing;

#[macro_use]
extern crate utoipa;

pub mod model;
mod postgres;
pub mod server;
mod services;
mod settings;
pub mod setup_tracing;

pub use settings::{CliOptions, Settings};

pub mod errors {

    use thiserror::Error;

    pub type BoxDynError = Box<dyn std::error::Error + 'static + Send + Sync>;

    #[derive(Debug, ToSchema, Error)]
    #[non_exhaustive]
    pub enum WeatherError {
        #[error("failed to convert GeoJson Feature: {0}")]
        GeoJson(#[from] geojson::Error),

        #[error("{target} expected missing GeoJson Feature property {property}")]
        MissingGeoJsonProperty { target: String, property: String },

        #[error("empty quantitative aggregation")]
        EmptyAggregation,

        #[error("missing GeoJson Feature:{0}")]
        MissingFeature(String),

        #[error("failed to parse Json: {0}")]
        Json(#[from] serde_json::Error),

        #[error("failed ro parse url: {0}")]
        UrlParse(#[from] url::ParseError),

        #[error("cannot extract location zone identifier from URL: {0}")]
        UrlNotZoneIdentifier(url::Url),

        // Api(#[from] server::ApiError),
        #[error("Encountered a technical failure: {source}")]
        Unexpected { source: anyhow::Error },
    }
}

pub(crate) async fn shutdown() {
    tokio::signal::ctrl_c().await.expect("failed to listen for signal event");
}
