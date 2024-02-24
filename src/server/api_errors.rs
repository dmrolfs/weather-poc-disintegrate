use thiserror::Error;

#[derive(Debug, Error, ToSchema)]
pub enum ApiError {
    #[error("failed to bootstrap server API: {0}")]
    Bootstrap(#[from] ApiBootstrapError),

    #[error("call to location registrar failed: {0}")]
    Registrar(#[from] crate::model::registrar::RegistrarError),
    // Registrar(crate::model::registrar::RegistrarFailure),
    #[error("call to update weather failed: {0}")]
    UpdateWeather(#[from] crate::model::weather::update::UpdateWeatherError),

    // #[error("{0}")]
    // ParseUrl(#[from] url::ParseError),
    #[error("{0}")]
    Noaa(#[from] crate::services::noaa::NoaaWeatherError),

    #[error("Invalid URL path input: {0}")]
    Path(#[from] axum::extract::rejection::PathRejection),

    #[error("Invalid JSON payload: {0}")]
    Json(#[from] axum::extract::rejection::JsonRejection),

    #[error("HTTP engine error: {0}")]
    HttpEngine(#[from] hyper::Error),

    #[error("failure during attempted database query: {source}")]
    Database { source: anyhow::Error },

    #[error("failed database operation: {0} ")]
    Sql(#[from] sqlx::Error),

    #[error("failed joining with thread: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),

    #[error("{0}")]
    IO(#[from] std::io::Error),
}

// impl From<crate::model::registrar::RegistrarFailure> for ApiError {
//     fn from(failure: crate::model::registrar::RegistrarFailure) -> Self {
//         Self::Registrar(failure)
//     }
// }

// impl From<cqrs_es::persist::PersistenceError> for ApiError {
//     fn from(error: cqrs_es::persist::PersistenceError) -> Self {
//         Self::Database { source: error.into() }
//     }
// }

#[derive(Debug, Error, ToSchema)]
pub enum ApiBootstrapError {
    #[error("failed to initialize Registrar subsystem: {0}")]
    Registrar(#[from] crate::model::registrar::RegistrarError),

    #[error("weather domain failure: {0}")]
    Weather(#[from] crate::model::weather::WeatherError),

    #[error("failed to initialize Location Zone subsystem: {0}")]
    LocationZone(#[from] crate::model::weather::zone::LocationZoneError),

    #[error("failed to initialize Update Locations subsystem: {0}")]
    UpdateLocations(#[from] crate::model::weather::update::UpdateWeatherError),

    #[error("invalid HTTP header value")]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    // #[error("failed to set up the journal postgres storage: {0}")]
    // Journal(#[from] coerce_cqrs::postgres::PostgresStorageError),
    #[error("failed to connect with NOAA weather service: {0}")]
    Noaa(#[from] crate::services::noaa::NoaaWeatherError),

    #[error("domain model postgres failure: {0}")]
    DomainPostgres(#[from] disintegrate_postgres::Error),

    #[error("{0}")]
    ParseUrl(#[from] url::ParseError),

    #[error("{0}")]
    IO(#[from] std::io::Error),
}
