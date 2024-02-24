use super::{LocationZoneCode, WeatherDecisionMakerRef};
use crate::model::{LocationZoneType, WeatherAlert};
use crate::services::noaa::ZoneWeatherApi;

mod protocol;
mod read_model;
mod services;
mod state;

pub use errors::LocationZoneError;
pub use read_model::WeatherRepository;
pub use support::LocationZoneSupport;

use crate::model::weather::update::UpdateWeatherId;
pub use read_model::ZONE_WEATHER_TABLE;

// pub type LocationZoneSerde = Json<LocationZoneEvent>;
// pub type LocationZoneEventStore = PgEventStore<LocationZoneEvent, LocationZoneSerde>;
// pub type LocationZoneDecisionMaker =
//     PgDecisionMaker<LocationZoneEvent, LocationZoneSerde, WithPgSnapshot>;
// pub type LocationZoneDecisionMakerRef = Arc<LocationZoneDecisionMaker>;

#[instrument(level = "debug", skip(weather_dm), err)]
pub async fn observe(
    update_id: UpdateWeatherId, zone: LocationZoneCode, weather_dm: WeatherDecisionMakerRef,
) -> Result<(), LocationZoneError> {
    let observation = services::services().zone_observation(&zone).await?;
    weather_dm
        .make(protocol::NoteObservation::new(zone, update_id, observation))
        .await
        .map_err(|err| LocationZoneError::Decision(Box::new(err)))?;
    Ok(())
}

#[instrument(level = "debug", skip(weather_dm), err)]
pub async fn forecast(
    update_id: UpdateWeatherId, zone: LocationZoneCode, weather_dm: WeatherDecisionMakerRef,
) -> Result<(), LocationZoneError> {
    let forecast = services::services()
        .zone_forecast(LocationZoneType::Forecast, &zone)
        .await?;
    weather_dm
        .make(protocol::NoteForecast::new(zone, update_id, forecast))
        .await
        .map_err(|err| LocationZoneError::Decision(Box::new(err)))?;
    Ok(())
}

#[instrument(level = "debug", skip(weather_dm), err)]
pub async fn alert(
    update_id: UpdateWeatherId, zone: LocationZoneCode, alert: Option<WeatherAlert>,
    weather_dm: WeatherDecisionMakerRef,
) -> Result<(), LocationZoneError> {
    weather_dm
        .make(protocol::NoteAlert::new(zone, update_id, alert))
        .await
        .map_err(|err| LocationZoneError::Decision(Box::new(err)))?;
    Ok(())
}

mod errors {
    // use coerce::actor::ActorId;
    use crate::errors::BoxDynError;
    use strum_macros::{Display, EnumDiscriminants};
    use thiserror::Error;

    #[derive(Debug, Error, EnumDiscriminants)]
    #[strum_discriminants(derive(Display, Serialize, Deserialize))]
    #[strum_discriminants(name(LocationZoneFailure))]
    pub enum LocationZoneError {
        #[error("{0}")]
        Noaa(#[from] crate::services::noaa::NoaaWeatherError),

        // #[error("failed to persist: {0}")]
        // Persist(#[from] coerce::persistent::PersistErr),
        #[error("failed to execute location zone decision: {0}")]
        Decision(#[source] BoxDynError),

        #[error("{0}")]
        JsonSerde(#[from] serde_json::Error),

        #[error("{0}")]
        Sql(#[from] sqlx::Error),

        #[error("{0}")]
        Postgres(#[from] disintegrate_postgres::Error),
        // #[error("failed to notify actor: {0}")]
        // ActorRef(#[from] coerce::actor::ActorRefErr),

        // #[error("failure in postgres storage: {0}")]
        // PostgresStorage(#[from] coerce_cqrs::postgres::PostgresStorageError),

        // #[error("{0}")]
        // Projection(#[from] coerce_cqrs::projection::ProjectionError),

        // #[error("ActorId cannot be used as LocationZoneId: {0}")]
        // BadActorId(ActorId),
    }

    // impl From<coerce::persistent::PersistErr> for LocationZoneFailure {
    //     fn from(error: coerce::persistent::PersistErr) -> Self {
    //         let zone_error: LocationZoneError = error.into();
    //         zone_error.into()
    //     }
    // }
}

mod support {
    use super::errors::LocationZoneError;
    use super::services::{LocationZoneServices, LocationZoneServicesRef};
    use crate::model::weather::zone::read_model::WeatherRepository;
    use crate::model::weather::WeatherEventStore;
    use crate::services::noaa::NoaaWeatherServices;
    use disintegrate_postgres::{PgEventListener, PgEventListenerConfig};
    use sqlx::PgPool;
    use std::sync::Arc;
    use std::time::Duration;

    #[derive(Debug, Clone)]
    pub struct LocationZoneSupport {
        // pub decision_maker: LocationZoneDecisionMakerRef,
        pub weather_repository: WeatherRepository,
        pub services: LocationZoneServicesRef,
    }

    impl LocationZoneSupport {
        pub async fn new(
            pool: PgPool, es: WeatherEventStore, noaa: NoaaWeatherServices,
        ) -> Result<Self, LocationZoneError> {
            // let serde = Json::<LocationZoneEvent>::default();
            // let event_store = PgEventStore::new(pool.clone(), serde).await?;
            // let decision_maker = Arc::new(disintegrate_postgres::decision_maker_with_snapshot(
            //     event_store.clone(),
            //     5,
            // ));

            let services = Arc::new(LocationZoneServices::new(noaa));

            let repo = WeatherRepository::new(pool.clone());
            let weather_projection = super::read_model::ZoneWeatherProjection::new(pool).await?;

            let listener_config = PgEventListenerConfig::poller(Duration::from_millis(50));
            PgEventListener::builder(es)
                .register_listener(weather_projection, listener_config)
                .start_with_shutdown(crate::shutdown())
                .await?;

            Ok(Self::direct(repo, services))
        }

        pub fn direct(
            weather_repository: WeatherRepository, services: LocationZoneServicesRef,
        ) -> Self {
            Self { weather_repository, services }
        }
    }
}
