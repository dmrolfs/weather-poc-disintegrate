mod location_status;
mod protocol;
mod read_model;
mod services;
mod state;
mod status;

pub use errors::UpdateWeatherError;
pub use read_model::{UpdateWeatherRepository, UpdateWeatherStatusView};
pub use services::{UpdateWeatherServices, UpdateWeatherServicesRef};
pub use state::UpdateWeatherId;
pub use support::UpdateWeatherSupport;

use crate::model::weather::update::protocol::{
    NoteAlertsReviewed, NoteLocationUpdateFailure, StartUpdate,
};
use crate::model::weather::{WeatherDecisionMakerRef, WeatherEvent};
use crate::model::LocationZoneCode;

#[instrument(level = "debug", skip(weather_dm), ret, err)]
pub async fn update_weather(
    zones: &[LocationZoneCode], weather_dm: WeatherDecisionMakerRef,
    services: UpdateWeatherServicesRef,
) -> Result<Option<UpdateWeatherId>, UpdateWeatherError> {
    if zones.is_empty() {
        return Ok(None);
    }

    let events = weather_dm
        .make(StartUpdate::for_zones(
            zones.to_vec(),
            weather_dm.clone(),
            services.clone(),
        )?)
        .await
        .map_err(|err| UpdateWeatherError::Decision(Box::new(err)))?;

    let update_id = events.into_iter().find_map(|pe| match pe.into_inner() {
        WeatherEvent::UpdateStarted { update_id, .. } => Some(update_id),
        _ => None,
    });

    Ok(update_id)
}

#[instrument(level = "debug", skip(weather_dm), err)]
pub async fn note_alerts_updated(
    update_id: UpdateWeatherId, weather_dm: WeatherDecisionMakerRef,
) -> Result<(), UpdateWeatherError> {
    weather_dm
        .make(NoteAlertsReviewed(update_id))
        .await
        .map_err(|err| UpdateWeatherError::Decision(Box::new(err)))?;

    Ok(())
}

#[instrument(level = "debug", skip(weather_dm), err)]
pub async fn note_zone_update_failure(
    update_id: UpdateWeatherId, zone: LocationZoneCode, failure: UpdateWeatherError,
    weather_dm: WeatherDecisionMakerRef,
) -> Result<(), UpdateWeatherError> {
    weather_dm
        .make(NoteLocationUpdateFailure { update_id, zone, cause: failure.to_string() })
        .await
        .map_err(|err| UpdateWeatherError::Decision(Box::new(err)))?;

    Ok(())
}

mod errors {
    use crate::errors::BoxDynError;
    use crate::model::weather::update::UpdateWeatherId;
    use crate::model::weather::zone::LocationZoneError;
    use crate::model::LocationZoneCode;
    use strum_macros::{Display, EnumDiscriminants};
    use thiserror::Error;

    #[derive(Debug, Error, EnumDiscriminants)]
    #[strum_discriminants(derive(Display, Serialize, Deserialize))]
    #[strum_discriminants(name(UpdateWeatherFailure))]
    pub enum UpdateWeatherError {
        #[error("no locations provided to update")]
        NoLocations,

        #[error("{0}")]
        Noaa(#[from] crate::services::noaa::NoaaWeatherError),

        #[error("update weather process [{0}] already started for zones: {1:?}")]
        AlreadyStarted(UpdateWeatherId, Vec<LocationZoneCode>),

        #[error("quiescent update weather process [{0}] cannot process command: {1}")]
        NotStarted(UpdateWeatherId, String),

        #[error("finished update weather process [{0}] cannot process command: {1}")]
        Finished(UpdateWeatherId, String),

        #[error("failed to execution update weather decision: {0}")]
        LocationZone(#[from] LocationZoneError),

        #[error("failed to execution update weather decision: {0}")]
        Decision(#[source] BoxDynError),

        #[error("SQL failure: {0}")]
        Sql(#[from] sqlx::Error),

        #[error("domain model postgres failure: {0}")]
        DomainPostgres(#[from] disintegrate_postgres::Error),

        #[error("{0}")]
        SerdeJson(#[from] serde_json::Error),

        // #[error("{0}")]
        // Connect(#[from] crate::connect::ConnectError),

        // #[error("failed to persist: {0}")]
        // Persist(#[from] coerce::persistent::PersistErr),

        // #[error("projection failure: {0}")]
        // Projection(#[from] coerce_cqrs::projection::ProjectionError),

        // #[error("failed to notify actor: {0}")]
        // ActorRef(#[from] coerce::actor::ActorRefErr),

        // #[error("ActorId cannot be used as UpdateLocationsId: {0}")]
        // BadActorId(ActorId),
        #[error("{0}")]
        ParseUrl(#[from] url::ParseError),
    }

    // impl From<coerce::persistent::PersistErr> for UpdateLocationsFailure {
    //     fn from(error: coerce::persistent::PersistErr) -> Self {
    //         let update_err: UpdateLocationsError = error.into();
    //         update_err.into()
    //     }
    // }
}

mod support {
    use crate::model::weather::update::read_model::UpdateWeatherRepository;
    use crate::model::weather::update::{
        UpdateWeatherError, UpdateWeatherServices, UpdateWeatherServicesRef,
    };
    use crate::model::weather::WeatherEventStore;
    use crate::services::noaa::NoaaWeatherServices;
    use anyhow::anyhow;
    use disintegrate_postgres::{PgEventListener, PgEventListenerConfig};
    use sqlx::PgPool;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio_util::task::TaskTracker;

    #[derive(Debug, Clone)]
    pub struct UpdateWeatherSupport {
        pub history_repository: UpdateWeatherRepository,
        pub services: UpdateWeatherServicesRef,
    }

    impl UpdateWeatherSupport {
        pub async fn from_noaa(
            pool: PgPool, es: WeatherEventStore, noaa: NoaaWeatherServices,
            task_tracker: &TaskTracker,
        ) -> Result<Self, UpdateWeatherError> {
            Self::new(
                pool,
                es,
                Arc::new(UpdateWeatherServices::new(noaa)),
                task_tracker,
            )
            .await
        }

        #[instrument(level = "debug", skip(es), err)]
        pub async fn new(
            pool: PgPool, es: WeatherEventStore, services: UpdateWeatherServicesRef,
            task_tracker: &TaskTracker,
        ) -> Result<Self, UpdateWeatherError> {
            let history_repository = UpdateWeatherRepository::new(pool.clone());

            task_tracker.spawn(async move {
                let update_history_projection =
                    super::read_model::UpdateWeatherHistoryProjection::new(pool).await?;
                let listener_config = PgEventListenerConfig::poller(Duration::from_millis(50));

                PgEventListener::builder(es)
                    .register_listener(update_history_projection, listener_config)
                    .start_with_shutdown(crate::shutdown())
                    .await
                    .map_err(|e| {
                        anyhow!("update history project event listener exited with error: {e}")
                    })?;
                Ok::<(), anyhow::Error>(())
            });

            Ok(Self { history_repository, services })
        }
    }
}
