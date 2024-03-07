use crate::model::registrar::errors::RegistrarError;
use crate::model::weather::update::{self, UpdateWeatherId, UpdateWeatherServicesRef};
use crate::model::weather::WeatherDecisionMakerRef;
use crate::model::LocationZoneCode;
use std::sync::Arc;

pub trait RegistrarApi: Sync + Send {
    // async fn initialize_forecast_zone(
    //     &self, zone: &LocationZoneCode, zone_dm: &LocationZoneDecisionMaker,
    // ) -> Result<(), RegistrarError>;

    async fn update_weather(
        &self, zones: &[LocationZoneCode], update_weather_dm: WeatherDecisionMakerRef,
    ) -> Result<Option<UpdateWeatherId>, RegistrarError>;
}

pub type RegistrarServicesRef = Arc<RegistrarServices>;

#[derive(Debug)]
pub enum RegistrarServices {
    Full(FullRegistrarServices),
    HappyPath(HappyPathServices),
}

impl RegistrarServices {
    #[allow(dead_code)]
    pub fn full(update_services: UpdateWeatherServicesRef) -> Self {
        Self::Full(FullRegistrarServices::new(update_services))
    }

    #[allow(dead_code)]
    pub const fn happy() -> Self {
        Self::HappyPath(HappyPathServices)
    }
}

impl RegistrarApi for RegistrarServices {
    // #[instrument(level = "debug", skip())]
    // async fn initialize_forecast_zone(
    //     &self, zone: &LocationZoneCode, dm: &LocationZoneDecisionMaker,
    // ) -> Result<(), RegistrarError> {
    //     match self {
    //         Self::Full(svc) => svc.initialize_forecast_zone(zone, dm).await,
    //         Self::HappyPath(svc) => svc.initialize_forecast_zone(zone, dm).await,
    //     }
    // }

    #[instrument(level = "debug", skip(self, dm), ret, err)]
    async fn update_weather(
        &self, zones: &[LocationZoneCode], dm: WeatherDecisionMakerRef,
    ) -> Result<Option<UpdateWeatherId>, RegistrarError> {
        match self {
            Self::Full(svc) => svc.update_weather(zones, dm).await,
            Self::HappyPath(svc) => svc.update_weather(zones, dm).await,
        }
    }
}

// static SERVICES: OnceCell<RegistrarServicesRef> = OnceCell::new();

/// Initializes the `RegistrarServices` used by the Registrar actor. This may be initialized
/// once, and will return the supplied value in an Err (i.e., `Err(services)`) on subsequent calls.
// pub fn initialize_services(services: RegistrarServicesRef) -> Result<(), RegistrarServicesRef> {
//     SERVICES.set(services)
// }

// pub fn services() -> RegistrarServicesRef {
//     SERVICES.get().expect("RegistrarServices are not initialized").clone()
// }

#[derive(Debug, Clone)]
pub struct FullRegistrarServices {
    update_services: UpdateWeatherServicesRef,
}

impl FullRegistrarServices {
    pub fn new(update_services: UpdateWeatherServicesRef) -> Self {
        Self { update_services }
    }
}

impl RegistrarApi for FullRegistrarServices {
    // async fn initialize_forecast_zone(
    //     &self, zone: &LocationZoneCode, dm: &LocationZoneDecisionMaker,
    // ) -> Result<(), RegistrarError> {
    //     zone_dm.make(StartLocationZoneMonitoring).await?;
    //     // let location_ref = zone::location_zone_for(zone, system).await?;
    //     // location_ref.notify(LocationZoneCommand::Start)?;
    //     Ok(())
    // }

    #[instrument(level = "debug", skip(self, dm), ret, err)]
    async fn update_weather(
        &self, zones: &[LocationZoneCode], dm: WeatherDecisionMakerRef,
    ) -> Result<Option<UpdateWeatherId>, RegistrarError> {
        if zones.is_empty() {
            return Ok(None);
        }

        update::update_weather(zones, dm, self.update_services.clone())
            .await
            .map_err(|err| err.into())
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct HappyPathServices;

impl RegistrarApi for HappyPathServices {
    // #[instrument(level = "debug", skip())]
    // async fn initialize_forecast_zone(
    //     &self, _zone: &LocationZoneCode, _dm: &ZoneDecisionMaker,
    // ) -> Result<(), RegistrarError> {
    //     Ok(())
    // }

    #[instrument(level = "debug", skip(self, _dm), ret, err)]
    async fn update_weather(
        &self, _zones: &[LocationZoneCode], _dm: WeatherDecisionMakerRef,
    ) -> Result<Option<UpdateWeatherId>, RegistrarError> {
        Ok(None)
    }
}
