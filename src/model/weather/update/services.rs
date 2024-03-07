use crate::model::WeatherAlert;
use crate::services::noaa::{AlertApi, NoaaWeatherError, NoaaWeatherServices};
use std::sync::Arc;

pub type UpdateWeatherServicesRef = Arc<UpdateWeatherServices>;

// static SERVICES: OnceCell<UpdateWeatherServicesRef> = OnceCell::new();

/// Initializes the `UpdateLocationServices` used by `UpdateLocations` actors. This may be
/// initialized once, and will return the supplied value in an Err (i.e., `Err(services)`) on subsequent calls.
// pub fn initialize_services(
//     services: UpdateWeatherServicesRef,
// ) -> Result<(), UpdateWeatherServicesRef> {
//     SERVICES.set(services)
// }

// pub fn services() -> UpdateWeatherServicesRef {
//     SERVICES.get().expect("UpdateWeatherServices is not initialized").clone()
// }

#[derive(Debug, Clone)]
pub struct UpdateWeatherServices {
    noaa: NoaaWeatherServices,
}

impl UpdateWeatherServices {
    pub fn new(noaa: NoaaWeatherServices) -> Self {
        Self { noaa }
    }
}

impl AlertApi for UpdateWeatherServices {
    async fn active_alerts(&self) -> Result<Vec<WeatherAlert>, NoaaWeatherError> {
        self.noaa.active_alerts().await
    }
}
