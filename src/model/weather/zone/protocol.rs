use crate::model::weather::update::UpdateWeatherId;
use crate::model::weather::zone::errors::LocationZoneError;
use crate::model::weather::zone::state::{
    LocationZoneAlert, LocationZoneForecast, LocationZoneWeather,
};
use crate::model::weather::WeatherEvent;
use crate::model::{LocationZoneCode, WeatherAlert, WeatherFrame, ZoneForecast};
use disintegrate::Decision;
use std::sync::Arc;

#[derive(Debug, PartialEq)]
pub struct NoteObservation {
    zone: LocationZoneCode,
    update_id: UpdateWeatherId,
    weather: Arc<WeatherFrame>,
}

impl NoteObservation {
    pub fn new(zone: LocationZoneCode, update_id: UpdateWeatherId, weather: WeatherFrame) -> Self {
        Self { zone, update_id, weather: Arc::new(weather) }
    }
}

impl Decision for NoteObservation {
    type Event = WeatherEvent;
    type StateQuery = LocationZoneWeather;
    type Error = LocationZoneError;

    fn state_query(&self) -> Self::StateQuery {
        LocationZoneWeather::new(self.zone.clone())
    }

    #[instrument(level = "debug", name = "NoteObservation::process", ret, err)]
    fn process(&self, _: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        Ok(vec![WeatherEvent::ObservationUpdated {
            zone: self.zone.clone(),
            update_id: self.update_id.clone(),
            weather: self.weather.clone(),
        }])
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct NoteForecast {
    zone: LocationZoneCode,
    update_id: UpdateWeatherId,
    forecast: Arc<ZoneForecast>,
}

impl NoteForecast {
    pub fn new(zone: LocationZoneCode, update_id: UpdateWeatherId, forecast: ZoneForecast) -> Self {
        Self { zone, update_id, forecast: Arc::new(forecast) }
    }
}

impl Decision for NoteForecast {
    type Event = WeatherEvent;
    type StateQuery = LocationZoneForecast;
    type Error = LocationZoneError;

    fn state_query(&self) -> Self::StateQuery {
        LocationZoneForecast::new(self.zone.clone())
    }

    #[instrument(level = "debug", name = "NoteForecast::process", ret, err)]
    fn process(&self, _: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        Ok(vec![WeatherEvent::ForecastUpdated {
            zone: self.zone.clone(),
            update_id: self.update_id.clone(),
            forecast: self.forecast.clone(),
        }])
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct NoteAlert {
    zone: LocationZoneCode,
    update_id: UpdateWeatherId,
    alert: Option<Arc<WeatherAlert>>,
}

impl NoteAlert {
    pub fn new(
        zone: LocationZoneCode, update_id: UpdateWeatherId, alert: Option<WeatherAlert>,
    ) -> Self {
        Self { zone, update_id, alert: alert.map(Arc::new) }
    }
}

impl Decision for NoteAlert {
    type Event = WeatherEvent;
    type StateQuery = LocationZoneAlert;
    type Error = LocationZoneError;

    fn state_query(&self) -> Self::StateQuery {
        LocationZoneAlert::new(self.zone.clone())
    }

    #[instrument(level = "debug", name = "NoteAlert::process", ret, err)]
    fn process(&self, state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        let event = match (state.active_alert(), &self.alert) {
            (false, Some(alert)) => Some(WeatherEvent::AlertActivated {
                zone: self.zone.clone(),
                update_id: self.update_id.clone(),
                alert: alert.clone(),
            }),
            (true, None) => Some(WeatherEvent::AlertDeactivated {
                zone: self.zone.clone(),
                update_id: self.update_id.clone(),
            }),
            _ => None,
        };

        let events: Vec<Self::Event> = event.into_iter().collect();
        Ok(events)
    }
}
