use super::state::Registrar;
use crate::model::registrar::errors::RegistrarError;
use crate::model::LocationZoneCode;
use disintegrate::{Decision, Event};
use strum_macros::Display;

#[derive(Debug, Display, Clone, PartialEq, Eq, Event, Serialize, Deserialize)]
// #[group(RegistrarEvent, [ForecastZoneAdded, ForecastZoneRemoved, AllForecastZonesRemoved])]
pub enum RegistrarEvent {
    ForecastZoneAdded { zone: LocationZoneCode },
    ForecastZoneRemoved { zone: LocationZoneCode },
    AllForecastZonesRemoved,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MonitorForecastZone(LocationZoneCode);

impl MonitorForecastZone {
    pub fn new(zone: LocationZoneCode) -> Self {
        Self(zone)
    }
}

impl Decision for MonitorForecastZone {
    type Event = RegistrarEvent;
    type StateQuery = Registrar;
    type Error = RegistrarError;

    fn state_query(&self) -> Self::StateQuery {
        Registrar::default()
    }

    #[instrument(level = "debug", name = "MonitorForecastZone::process", ret, err)]
    fn process(&self, state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        let zone = &self.0;

        if state.location_codes.contains(zone) {
            return Err(RegistrarError::LocationZoneAlreadyMonitored(zone.clone()));
        }

        Ok(vec![RegistrarEvent::ForecastZoneAdded {
            zone: self.0.clone(),
        }])
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IgnoreForecastZone(LocationZoneCode);

impl IgnoreForecastZone {
    pub fn new(zone: LocationZoneCode) -> Self {
        Self(zone)
    }
}

impl Decision for IgnoreForecastZone {
    type Event = RegistrarEvent;
    type StateQuery = Registrar;
    type Error = RegistrarError;

    fn state_query(&self) -> Self::StateQuery {
        Registrar::default()
    }

    #[instrument(level = "debug", name = "ForgetForecastZone::process", ret, err)]
    fn process(&self, state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        let zone = &self.0;

        if state.location_codes.contains(zone) {
            Ok(vec![RegistrarEvent::ForecastZoneRemoved {
                zone: zone.clone(),
            }])
        } else {
            Ok(Vec::default())
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ClearZoneMonitoring;

impl Decision for ClearZoneMonitoring {
    type Event = RegistrarEvent;
    type StateQuery = Registrar;
    type Error = RegistrarError;

    fn state_query(&self) -> Self::StateQuery {
        Registrar::default()
    }

    fn process(&self, state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        if !state.location_codes.is_empty() {
            Ok(vec![RegistrarEvent::AllForecastZonesRemoved])
        } else {
            Ok(Vec::default())
        }
    }
}
