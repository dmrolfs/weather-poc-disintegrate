use crate::model::LocationZoneCode;
use disintegrate::serde::json::Json;
use disintegrate::NoSnapshot;
use disintegrate_postgres::{PgDecisionMaker, PgEventStore};
use std::sync::Arc;

// mod processor;
pub mod protocol;
mod read_model;
mod services;
mod state;

use crate::model::registrar::protocol::RegistrarEvent;
pub use errors::RegistrarError;
pub use read_model::{MonitoredLocationZonesRef, MonitoredLocationZonesView};
pub use services::RegistrarServices;

pub type RegistrarEventSerde = Json<protocol::RegistrarEvent>;
pub type RegistrarEventStore = PgEventStore<RegistrarEvent, RegistrarEventSerde>;
pub type RegistrarDecisionMaker = PgDecisionMaker<RegistrarEvent, RegistrarEventSerde, NoSnapshot>;
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
    }
}

pub mod support {
    use super::services::RegistrarServicesRef;
    use super::{
        MonitoredLocationZonesRef, RegistrarDecisionMakerRef, RegistrarError, RegistrarEventStore,
        RegistrarServices,
    };
    use crate::model::weather::update::UpdateWeatherServicesRef;
    use anyhow::anyhow;
    use disintegrate_postgres::{PgEventListener, PgEventListenerConfig};
    use std::fmt;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio_util::task::TaskTracker;

    #[derive(Clone)]
    pub struct RegistrarSupport {
        pub decision_maker: RegistrarDecisionMakerRef,
        pub event_store: RegistrarEventStore,
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
        #[instrument(
            level = "debug",
            name = "RegistrarSupport::new",
            skip(event_store),
            err
        )]
        pub async fn new(
            event_store: RegistrarEventStore, update_services: UpdateWeatherServicesRef,
            task_tracker: &TaskTracker,
        ) -> Result<Self, RegistrarError> {
            let decision_maker =
                Arc::new(disintegrate_postgres::decision_maker(event_store.clone()));
            warn!("DMR: RS-AAA");

            let services = Arc::new(RegistrarServices::full(update_services));

            let monitored = super::read_model::MonitoredLocationZones::default();
            warn!("DMR: RS-BBB");

            // let registrar_processor = Arc::new(registrar::processor::RegistrarProcessor::new());
            let event_store_0 = event_store.clone();
            let monitored_0 = monitored.clone();

            task_tracker.spawn(async move {
                PgEventListener::builder(event_store_0)
                    .register_listener(
                        monitored_0,
                        PgEventListenerConfig::poller(Duration::from_millis(50)),
                    )
                    .start_with_shutdown(crate::shutdown())
                    .await
                    .map_err(|e| {
                        anyhow!("registrar zone monitor event listener exited with error: {e}")
                    })?;
                Ok::<(), anyhow::Error>(())
            });
            warn!("DMR: RS-CCC");

            Ok(Self {
                decision_maker,
                event_store,
                monitored: Arc::new(monitored),
                services,
            })
        }
    }
}
