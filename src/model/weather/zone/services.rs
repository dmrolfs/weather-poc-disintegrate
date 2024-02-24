use crate::model::{LocationZoneCode, LocationZoneType, WeatherFrame, ZoneForecast};
use crate::services::noaa::{NoaaWeatherError, NoaaWeatherServices, ZoneWeatherApi};
use once_cell::sync::OnceCell;
use std::sync::Arc;

pub type LocationZoneServicesRef = Arc<LocationZoneServices>;

static SERVICES: OnceCell<LocationZoneServicesRef> = OnceCell::new();

/// Initializes the `LocationServices` used by LocationZone actors. This may be initialized
/// once, and will return the supplied value in an Err (i.e., `Err(services)`) on subsequent calls.
pub fn initialize_services(
    services: LocationZoneServicesRef,
) -> Result<(), LocationZoneServicesRef> {
    SERVICES.set(services)
}

pub fn services() -> LocationZoneServicesRef {
    SERVICES.get().expect("LocationZoneServices are not initialized").clone()
}

#[derive(Debug, Clone)]
pub struct LocationZoneServices(NoaaWeatherServices);

impl LocationZoneServices {
    pub fn new(noaa: NoaaWeatherServices) -> Self {
        Self(noaa)
    }
}

impl ZoneWeatherApi for LocationZoneServices {
    async fn zone_observation(
        &self, zone: &LocationZoneCode,
    ) -> Result<WeatherFrame, NoaaWeatherError> {
        self.0.zone_observation(zone).await
    }

    async fn zone_forecast(
        &self, zone_type: LocationZoneType, zone: &LocationZoneCode,
    ) -> Result<ZoneForecast, NoaaWeatherError> {
        self.0.zone_forecast(zone_type, zone).await
    }
}
