use crate::model::LocationZoneCode;
use disintegrate::serde::json::Json;
use disintegrate::NoSnapshot;
use disintegrate_postgres::PgDecisionMaker;
use std::sync::Arc;

// mod processor;
pub mod protocol;
mod read_model;
mod services;
mod state;

pub use errors::RegistrarError;
pub use read_model::{MonitoredLocationZonesRef, MonitoredLocationZonesView};
pub use services::RegistrarServices;

pub type RegistrarSerde = Json<protocol::RegistrarEvent>;
pub type RegistrarDecisionMaker =
    PgDecisionMaker<protocol::RegistrarEvent, RegistrarSerde, NoSnapshot>;
pub type RegistrarDecisionMakerRef = Arc<RegistrarDecisionMaker>;

#[instrument(level = "debug", skip(dm), err)]
pub async fn clear_monitoring(dm: &RegistrarDecisionMaker) -> Result<(), RegistrarError> {
    dm.make(protocol::ClearZoneMonitoring)
        .await
        .map_err(|err| RegistrarError::Decision(Box::new(err)))?;
    Ok(())
}

#[instrument(level = "debug", skip(dm), err)]
pub async fn monitor_forecast_zone(
    zone: LocationZoneCode, dm: &RegistrarDecisionMaker,
) -> Result<(), RegistrarError> {
    dm.make(protocol::MonitorForecastZone::new(zone.clone()))
        .await
        .map_err(|err| RegistrarError::Decision(Box::new(err)))?;
    // let _handle = MonitorForecastZone::then_run(&zone, support);
    Ok(())
}

#[instrument(level = "debug", skip(dm), err)]
pub async fn ignore_forecast_zone(
    zone: LocationZoneCode, dm: &RegistrarDecisionMaker,
) -> Result<(), RegistrarError> {
    dm.make(protocol::IgnoreForecastZone::new(zone))
        .await
        .map_err(|err| RegistrarError::Decision(Box::new(err)))?;
    Ok(())
}

// fn result_from<T, E>(command_result: CommandResult<T, E>) -> Result<T, RegistrarFailure>
//     where
//         E: std::fmt::Display + Into<RegistrarFailure>,
// {
//     match command_result {
//         CommandResult::Ok(x) => Ok(x),
//         CommandResult::Rejected(msg) => Err(RegistrarError::RejectedCommand(msg).into()),
//         CommandResult::Err(error) => Err(error.into()),
//     }
// }

mod errors {
    use crate::errors::BoxDynError;
    use crate::model::LocationZoneCode;
    use strum_macros::{Display, EnumDiscriminants};
    use thiserror::Error;

    #[derive(Debug, Error, EnumDiscriminants)]
    #[strum_discriminants(derive(Display, Serialize, Deserialize))]
    #[strum_discriminants(name(RegistrarFailure))]
    pub enum RegistrarError {
        #[error("already monitoring location zone code: {0}")]
        LocationZoneAlreadyMonitored(LocationZoneCode),

        #[error("{0}")]
        LocationZone(#[from] crate::model::weather::zone::LocationZoneError),

        #[error("{0}")]
        UpdateWeather(#[from] crate::model::weather::update::UpdateWeatherError),

        #[error("failed to execute registrar decision: {0}")]
        Decision(#[source] BoxDynError),

        #[error("{0}")]
        Postgres(#[from] disintegrate_postgres::Error),
        // #[error("{0}")]
        // ActorRef(#[from] coerce::actor::ActorRefErr),

        // #[error("failed to persist: {0}")]
        // Persist(#[from] coerce::persistent::PersistErr),

        // #[error("failure in postgres storage: {0}")]
        // PostgresStorage(#[from] coerce_cqrs::postgres::PostgresStorageError),

        // #[error("projection failure: {0}")]
        // Projection(#[from] coerce_cqrs::projection::ProjectionError),

        // #[error("command rejected: {0}")]
        // RejectedCommand(String),
    }

    // impl From<coerce::actor::ActorRefErr> for RegistrarFailure {
    //     fn from(error: coerce::actor::ActorRefErr) -> Self {
    //         let reg_error: RegistrarError = error.into();
    //         reg_error.into()
    //     }
    // }

    // impl From<coerce::persistent::PersistErr> for RegistrarFailure {
    //     fn from(error: coerce::persistent::PersistErr) -> Self {
    //         let reg_error: RegistrarError = error.into();
    //         reg_error.into()
    //     }
    // }
}

pub mod support {
    use super::protocol::RegistrarEvent;
    use super::services::RegistrarServicesRef;
    use super::{
        MonitoredLocationZonesRef, RegistrarDecisionMakerRef, RegistrarError, RegistrarServices,
    };
    use crate::model::weather::update::UpdateWeatherServicesRef;
    use disintegrate::serde::json::Json;
    use disintegrate_postgres::{PgEventListener, PgEventListenerConfig, PgEventStore};
    use sqlx::PgPool;
    use std::fmt;
    use std::sync::Arc;
    use std::time::Duration;

    #[derive(Clone)]
    pub struct RegistrarSupport {
        pub decision_maker: RegistrarDecisionMakerRef,
        // pub zone_decision_maker: ZoneDecisionMaker,
        // pub update_weather_decision_maker: UpdateWeatherDecisionMaker,
        pub monitored: MonitoredLocationZonesRef,
        pub services: RegistrarServicesRef,
    }

    impl fmt::Debug for RegistrarSupport {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("RegistrarSupport")
                .field("services", &self.services)
                .finish()
        }
    }

    impl RegistrarSupport {
        pub async fn new(
            pool: PgPool, update_services: UpdateWeatherServicesRef,
        ) -> Result<Self, RegistrarError> {
            let serde = Json::<RegistrarEvent>::default();
            let event_store = PgEventStore::new(pool, serde).await?;
            let decision_maker =
                Arc::new(disintegrate_postgres::decision_maker(event_store.clone()));

            let services = Arc::new(RegistrarServices::full(update_services));

            let monitored = super::read_model::MonitoredLocationZones::default();

            // let registrar_processor = Arc::new(registrar::processor::RegistrarProcessor::new());
            PgEventListener::builder(event_store)
                .register_listener(
                    monitored.clone(),
                    PgEventListenerConfig::poller(Duration::from_millis(50)),
                )
                // .register_listener(
                //     registrar_processor.clone(),
                //     PgEventListenerConfig::poller(Duration::from_millis(50)),
                // )
                .start_with_shutdown(crate::shutdown())
                .await?;

            Ok(Self {
                decision_maker,
                monitored: Arc::new(monitored),
                services,
            })
        }
    }
}
