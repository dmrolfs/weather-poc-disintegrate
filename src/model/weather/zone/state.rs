use crate::model::weather::LocationZoneEvent;
use crate::model::{LocationZoneCode, WeatherAlert, WeatherFrame, ZoneForecast};
use disintegrate::{StateMutate, StateQuery};
use std::sync::Arc;
use tagid::Label;

#[derive(Debug, Clone, StateQuery, Label, Serialize, Deserialize)]
#[state_query(LocationZoneEvent)]
pub struct LocationZoneWeather {
    #[id]
    zone: LocationZoneCode,
    weather: Option<Arc<WeatherFrame>>,
}

impl LocationZoneWeather {
    pub fn new(zone: LocationZoneCode) -> Self {
        Self { zone, weather: None }
    }
}

impl StateMutate for LocationZoneWeather {
    fn mutate(&mut self, event: Self::Event) {
        if let LocationZoneEvent::ObservationUpdated { weather, .. } = event {
            self.weather = Some(weather);
        }
    }
}

#[derive(Debug, Clone, StateQuery, Label, Serialize, Deserialize)]
#[state_query(LocationZoneEvent)]
pub struct LocationZoneForecast {
    #[id]
    zone: LocationZoneCode,
    forecast: Option<Arc<ZoneForecast>>,
}

impl LocationZoneForecast {
    pub fn new(zone: LocationZoneCode) -> Self {
        Self { zone, forecast: None }
    }
}

impl StateMutate for LocationZoneForecast {
    fn mutate(&mut self, event: Self::Event) {
        if let LocationZoneEvent::ForecastUpdated { forecast, .. } = event {
            self.forecast = Some(forecast);
        }
    }
}

#[derive(Debug, Clone, StateQuery, Label, Serialize, Deserialize)]
#[state_query(LocationZoneEvent)]
pub struct LocationZoneAlert {
    #[id]
    zone: LocationZoneCode,
    alert: Option<Arc<WeatherAlert>>,
}

impl LocationZoneAlert {
    pub fn new(zone: LocationZoneCode) -> Self {
        Self { zone, alert: None }
    }

    #[inline]
    pub fn active_alert(&self) -> bool {
        self.alert.is_some()
    }
}

impl StateMutate for LocationZoneAlert {
    fn mutate(&mut self, event: Self::Event) {
        match event {
            LocationZoneEvent::AlertActivated { alert, .. } => {
                self.alert = Some(alert);
            },
            LocationZoneEvent::AlertDeactivated { .. } => {
                self.alert = None;
            },
            _ => {},
        }
    }
}
