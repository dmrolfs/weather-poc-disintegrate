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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;
    use claims::assert_matches;
    use once_cell::sync::Lazy;
    use RegistrarEvent as E;

    static OTIS: Lazy<LocationZoneCode> = Lazy::new(|| LocationZoneCode::new("otis"));
    static STELLA: Lazy<LocationZoneCode> = Lazy::new(|| LocationZoneCode::new("stella"));
    static NEO: Lazy<LocationZoneCode> = Lazy::new(|| LocationZoneCode::new("neo"));

    #[test]
    fn it_adds_zone_to_monitor() {
        testing::TestHarness::given([])
            .when(MonitorForecastZone::new(OTIS.clone()))
            .then([E::ForecastZoneAdded { zone: OTIS.clone() }]);
    }

    #[test]
    fn it_should_not_add_zone_that_is_already_monitored() {
        let err = testing::TestHarness::given([E::ForecastZoneAdded { zone: OTIS.clone() }])
            .when(MonitorForecastZone::new(OTIS.clone()))
            .then_err();
        assert_matches!(err, RegistrarError::LocationZoneAlreadyMonitored(zone) if zone == OTIS.clone());
    }

    #[test]
    fn it_removes_zone_from_monitoring() {
        testing::TestHarness::given([E::ForecastZoneAdded { zone: OTIS.clone() }])
            .when(IgnoreForecastZone::new(OTIS.clone()))
            .then([E::ForecastZoneRemoved { zone: OTIS.clone() }]);
    }

    #[test]
    fn it_should_ignore_request_to_remove_zone_that_is_not_monitored() {
        testing::TestHarness::given([E::ForecastZoneAdded { zone: OTIS.clone() }])
            .when(IgnoreForecastZone::new(STELLA.clone()))
            .then([]);
    }

    #[test]
    fn it_should_clear_all_zone_monitoring() {
        testing::TestHarness::given([
            E::ForecastZoneAdded { zone: OTIS.clone() },
            E::ForecastZoneAdded { zone: STELLA.clone() },
            E::ForecastZoneAdded { zone: NEO.clone() },
        ])
        .when(ClearZoneMonitoring)
        .then([E::AllForecastZonesRemoved])
    }
}
