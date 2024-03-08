pub mod update;
pub mod zone;

pub use errors::WeatherError;
pub use support::WeatherSupport;

use self::update::UpdateWeatherId;
use crate::model::{LocationZoneCode, WeatherAlert, WeatherFrame, ZoneForecast};
use disintegrate::serde::json::Json;
use disintegrate::Event;
use disintegrate_postgres::{PgDecisionMaker, PgEventStore, WithPgSnapshot};
use std::sync::Arc;

pub type WeatherEventSerde = Json<WeatherEvent>;
pub type WeatherEventStore = PgEventStore<WeatherEvent, WeatherEventSerde>;
pub type WeatherDecisionMaker = PgDecisionMaker<WeatherEvent, WeatherEventSerde, WithPgSnapshot>;
pub type WeatherDecisionMakerRef = Arc<WeatherDecisionMaker>;

#[derive(Debug, Clone, PartialEq, Eq, Event, Serialize, Deserialize)]
#[group(LocationZoneEvent, [ObservationUpdated, ForecastUpdated, AlertActivated, AlertDeactivated])]
#[group(UpdateWeatherEvent, [UpdateStarted, AlertsReviewed, UpdateLocationFailed, UpdateCompleted, UpdateFailed])]
pub enum WeatherEvent {
    ObservationUpdated {
        #[id]
        zone: LocationZoneCode,
        #[id]
        update_id: UpdateWeatherId,
        weather: Arc<WeatherFrame>,
    },
    ForecastUpdated {
        #[id]
        zone: LocationZoneCode,
        #[id]
        update_id: UpdateWeatherId,
        forecast: Arc<ZoneForecast>,
    },
    AlertActivated {
        #[id]
        zone: LocationZoneCode,
        #[id]
        update_id: UpdateWeatherId,
        alert: Arc<WeatherAlert>,
    },
    AlertDeactivated {
        #[id]
        zone: LocationZoneCode,
        #[id]
        update_id: UpdateWeatherId,
    },
    UpdateStarted {
        #[id]
        update_id: UpdateWeatherId,
        zones: Vec<LocationZoneCode>,
    },
    AlertsReviewed {
        #[id]
        update_id: UpdateWeatherId,
    },
    UpdateLocationFailed {
        #[id]
        update_id: UpdateWeatherId,
        #[id]
        zone: LocationZoneCode,
        cause: String,
    },
    // UpdateCompleted {
    //     #[id]
    //     update_id: UpdateWeatherId,
    // },
    // UpdateFailed {
    //     #[id]
    //     update_id: UpdateWeatherId,
    // },
}

impl WeatherEvent {
    #[inline]
    pub fn update_id(&self) -> &UpdateWeatherId {
        match self {
            Self::AlertActivated { update_id, .. } => update_id,
            Self::AlertDeactivated { update_id, .. } => update_id,
            Self::AlertsReviewed { update_id, .. } => update_id,
            Self::ForecastUpdated { update_id, .. } => update_id,
            Self::UpdateLocationFailed { update_id, .. } => update_id,
            Self::ObservationUpdated { update_id, .. } => update_id,
            Self::UpdateStarted { update_id, .. } => update_id,
        }
    }

    #[inline]
    pub fn zones(&self) -> Vec<LocationZoneCode> {
        match self {
            Self::AlertActivated { zone, .. } => vec![zone.clone()],
            Self::AlertDeactivated { zone, .. } => vec![zone.clone()],
            Self::AlertsReviewed { .. } => vec![],
            Self::ForecastUpdated { zone, .. } => vec![zone.clone()],
            Self::UpdateLocationFailed { zone, .. } => vec![zone.clone()],
            Self::ObservationUpdated { zone, .. } => vec![zone.clone()],
            Self::UpdateStarted { zones, .. } => zones.clone(),
        }
    }
}

mod errors {
    use strum_macros::{Display, EnumDiscriminants};
    use thiserror::Error;

    #[derive(Debug, Error, EnumDiscriminants)]
    #[strum_discriminants(derive(Display, Serialize, Deserialize))]
    #[strum_discriminants(name(WeatherFailure))]
    pub enum WeatherError {
        #[error("domain model postgres failure: {0}")]
        DomainPostgres(#[from] disintegrate_postgres::Error),
    }
}

mod support {
    use super::{WeatherDecisionMakerRef, WeatherError, WeatherEventStore};
    use std::sync::Arc;

    #[derive(Clone)]
    pub struct WeatherSupport {
        pub decision_maker: WeatherDecisionMakerRef,
        pub event_store: WeatherEventStore,
    }

    impl WeatherSupport {
        #[instrument(level = "debug", name = "WeatherSupport::new", skip(es), err)]
        pub async fn new(es: WeatherEventStore) -> Result<Self, WeatherError> {
            let dm =
                Arc::new(disintegrate_postgres::decision_maker_with_snapshot(es.clone(), 5).await?);
            Ok(Self::direct(dm, es))
        }

        pub fn direct(
            decision_maker: WeatherDecisionMakerRef, event_store: WeatherEventStore,
        ) -> Self {
            Self { decision_maker, event_store }
        }
    }
}
