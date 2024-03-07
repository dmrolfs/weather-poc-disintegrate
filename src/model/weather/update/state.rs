use super::location_status::{LocationStatus, MultiIndexLocationStatusMap};
use super::status::UpdateStep;
use crate::model::weather::update::status::LocationUpdateStatus;
use crate::model::weather::WeatherEvent;
use crate::model::LocationZoneCode;
use disintegrate::{StateMutate, StateQuery};
use std::collections::HashSet;
use std::str::FromStr;
use strum_macros::{EnumDiscriminants, EnumString};
use tagid::{CuidId, Label};

pub type UpdateWeatherId = CuidId<UpdateWeather>;

#[derive(Debug, Clone, PartialEq, Label, StateQuery, Serialize, Deserialize)]
#[state_query(WeatherEvent)]
pub struct UpdateWeather {
    #[id]
    pub update_id: UpdateWeatherId,
    pub state: UpdateWeatherState,
}

impl UpdateWeather {
    pub fn new(update_id: UpdateWeatherId) -> Self {
        Self { update_id, state: UpdateWeatherState::default() }
    }
}

impl StateMutate for UpdateWeather {
    fn mutate(&mut self, event: Self::Event) {
        if let Some(new_state) = self.state.mutate(event) {
            self.state = new_state;
        }
    }
}

impl tagid::Entity for UpdateWeather {
    type IdGen = tagid::CuidGenerator;
}

#[derive(Debug, Clone, PartialEq, EnumDiscriminants, Serialize, Deserialize)]
#[strum_discriminants(derive(strum_macros::Display, EnumString, Serialize))]
pub enum UpdateWeatherState {
    Quiescent(QuiescentWeatherUpdate),
    Active(WeatherUpdateStatus),
    Finished(FinishedWeatherUpdate),
}

impl Default for UpdateWeatherState {
    fn default() -> Self {
        Self::Quiescent(QuiescentWeatherUpdate)
    }
}

impl UpdateWeatherState {
    #[instrument(level = "debug", ret)]
    fn mutate(&mut self, event: WeatherEvent) -> Option<Self> {
        match self {
            Self::Quiescent(q) => q.mutate(event),
            Self::Active(a) => match a.mutate(event) {
                UpdateWeatherStateDiscriminants::Active => None,
                UpdateWeatherStateDiscriminants::Finished => {
                    Some(UpdateWeatherState::Finished(FinishedWeatherUpdate))
                },
                UpdateWeatherStateDiscriminants::Quiescent => {
                    error!("quiescent state mutation not possible by event ");
                    None
                },
            },
            Self::Finished(f) => f.mutate(event),
        }
    }
}

impl<'q, DB> sqlx::Decode<'q, DB> for UpdateWeatherStateDiscriminants
where
    String: sqlx::Decode<'q, DB>,
    DB: sqlx::Database,
{
    fn decode(
        value: <DB as sqlx::database::HasValueRef<'q>>::ValueRef,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let value = <String as sqlx::Decode<DB>>::decode(value)?;
        let value = UpdateWeatherStateDiscriminants::from_str(&value)?;
        Ok(value)
    }
}

impl<'q, DB> sqlx::Encode<'q, DB> for UpdateWeatherStateDiscriminants
where
    String: sqlx::Encode<'q, DB>,
    DB: sqlx::Database,
{
    fn encode_by_ref(
        &self, buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        <String as sqlx::Encode<DB>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl<DB> sqlx::Type<DB> for UpdateWeatherStateDiscriminants
where
    String: sqlx::Type<DB>,
    DB: sqlx::Database,
{
    fn type_info() -> DB::TypeInfo {
        <String as sqlx::Type<DB>>::type_info()
    }
}

#[derive(Debug, Default, Clone, PartialEq, ToSchema, Serialize, Deserialize)]
pub struct QuiescentWeatherUpdate;

impl QuiescentWeatherUpdate {
    fn mutate(&mut self, event: WeatherEvent) -> Option<UpdateWeatherState> {
        match event {
            WeatherEvent::UpdateStarted { zones, .. } => {
                Some(UpdateWeatherState::Active(WeatherUpdateStatus::new(zones)))
            },
            event => {
                warn!(
                    ?event,
                    "quiescent update weather process cannot handle event before starting"
                );
                None
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, ToSchema, Serialize, Deserialize)]
pub struct WeatherUpdateStatus {
    location_statuses: MultiIndexLocationStatusMap,
    pub alerts_reviewed: bool,
}

impl WeatherUpdateStatus {
    pub fn new(zones: Vec<LocationZoneCode>) -> Self {
        let mut location_statuses = MultiIndexLocationStatusMap::with_capacity(zones.len());
        zones.into_iter().for_each(|zone| {
            location_statuses.insert(LocationStatus::new(zone));
        });
        // for zone in zones {
        //     location_statuses.insert(LocationStatus::new(zone));
        // }

        Self { location_statuses, alerts_reviewed: false }
    }

    #[inline]
    pub fn status_for(&self, zone: &LocationZoneCode) -> Option<LocationUpdateStatus> {
        self.location_statuses.get_by_zone(zone).map(|ls| ls.status)
    }

    #[instrument(level = "debug", skip(self), ret)]
    pub fn active_zones(&self) -> HashSet<LocationZoneCode> {
        let mut result = HashSet::new();
        for ls in self.location_statuses.iter() {
            if let LocationUpdateStatus::InProgress(_) = &ls.1.status {
                result.insert(ls.1.zone.clone());
            }
        }
        result
    }

    #[instrument(level = "debug", skip(self), ret)]
    pub fn succeeded_zones(&self) -> HashSet<LocationZoneCode> {
        let mut result = HashSet::new();
        for ls in self.location_statuses.iter() {
            if ls.1.status == LocationUpdateStatus::Succeeded {
                result.insert(ls.1.zone.clone());
            }
        }
        result
    }

    #[instrument(level = "debug", skip(self), ret)]
    pub fn failed_zones(&self) -> HashSet<LocationZoneCode> {
        let mut result = HashSet::new();
        for ls in self.location_statuses.iter() {
            if ls.1.status == LocationUpdateStatus::Failed {
                result.insert(ls.1.zone.clone());
            }
        }
        result
    }

    #[instrument(level = "debug", ret)]
    pub fn mutate(&mut self, event: WeatherEvent) -> UpdateWeatherStateDiscriminants {
        use WeatherEvent as E;

        match event {
            E::ObservationUpdated { zone, .. } => {
                self.advance_zone_step(&zone, UpdateStep::Observation)
            },

            E::ForecastUpdated { zone, .. } => self.advance_zone_step(&zone, UpdateStep::Forecast),

            E::AlertActivated { zone, .. } | E::AlertDeactivated { zone, .. } => {
                self.advance_zone_step(&zone, UpdateStep::Alert)
            },

            E::AlertsReviewed { .. } => {
                self.alerts_reviewed = true;
                if self.active_zones().is_empty() {
                    UpdateWeatherStateDiscriminants::Finished
                } else {
                    UpdateWeatherStateDiscriminants::Active
                }
            },

            E::UpdateLocationFailed { zone, cause, .. } => {
                self.update_zone_failure_for(zone, cause)
            },

            // E::UpdateCompleted { .. } | E::UpdateFailed { .. } => {
            //     Some(UpdateWeatherState::Finished(FinishedWeatherUpdate))
            // },
            event => {
                warn!(?event, "active update weather process cannot handle event");
                UpdateWeatherStateDiscriminants::Active
            },
        }
    }

    #[instrument(level = "trace", ret)]
    pub fn is_only_active_zone(&self, zone: &LocationZoneCode) -> bool {
        let active_zone = self
            .location_statuses
            .get_by_zone(zone)
            .map(|ls| ls.status.is_active())
            .unwrap_or_default();

        active_zone && self.location_statuses
            .iter_by_status()
            .filter(|ls| &ls.zone != zone)
            .all(|ls| {
            trace!(
                location_statuses=%ls.status, %zone,
                "DMR: is {location} active:{is_active}; does {location} match {zone}:{does_match}",
                is_active=ls.status.is_completed(), does_match=zone == &ls.zone, location=ls.zone,
            );
            ls.status.is_completed()
        })
    }

    fn prep_zone(&mut self, zone: &LocationZoneCode) -> Option<UpdateWeatherStateDiscriminants> {
        match self.status_for(zone) {
            None => {
                info!("adding new zone to update status: {zone}");
                self.location_statuses.insert(LocationStatus::new(zone.clone()));
                None
            },

            Some(status) if status.is_completed() => {
                warn!("{zone} zone was marked for update after completion - ignored");
                Some(UpdateWeatherStateDiscriminants::Finished)
            },

            _ => None,
        }
    }

    pub fn advance_zone_step(
        &mut self, zone: &LocationZoneCode, step: UpdateStep,
    ) -> UpdateWeatherStateDiscriminants {
        if let Some(finished) = self.prep_zone(zone) {
            return finished;
        }

        let is_only_active_zone = self.is_only_active_zone(zone);
        let ls: &LocationStatus = self
            .location_statuses
            .modify_by_zone(zone, |ls| ls.status.advance(step))
            .unwrap();
        let is_completed = self.alerts_reviewed && is_only_active_zone && ls.status.is_completed();

        debug!(
            advanced_status=%ls.status,
            alerts_reviewed=%self.alerts_reviewed, is_status_completed=%ls.status.is_completed(), %is_only_active_zone,
            "location {zone} update {} saga", if is_completed { "completes" } else { "does not complete"}
        );

        if is_completed {
            UpdateWeatherStateDiscriminants::Finished
        } else {
            UpdateWeatherStateDiscriminants::Active
        }
    }

    #[instrument(level = "debug", ret)]
    pub fn update_zone_failure_for(
        &mut self, zone: LocationZoneCode, cause: String,
    ) -> UpdateWeatherStateDiscriminants {
        if let Some(finished) = self.prep_zone(&zone) {
            return finished;
        }

        let is_only_active_zone = self.is_only_active_zone(&zone);

        let ls_zone = zone.clone();
        self.location_statuses.modify_by_zone(&zone, |ls| {
            *ls = LocationStatus {
                zone: ls_zone,
                status: LocationUpdateStatus::failed(),
            };
        });

        let is_completed = self.alerts_reviewed && is_only_active_zone;
        debug!(
            failed_status=?self.status_for(&zone),
            alerts_reviewed=%self.alerts_reviewed,
            %is_only_active_zone,
            "location {zone} failure update {} saga", if is_completed { "completes" } else { "does not complete"}
        );

        if is_completed {
            UpdateWeatherStateDiscriminants::Finished
        } else {
            UpdateWeatherStateDiscriminants::Active
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, ToSchema, Serialize, Deserialize)]
pub struct FinishedWeatherUpdate;

impl FinishedWeatherUpdate {
    fn mutate(&mut self, event: WeatherEvent) -> Option<UpdateWeatherState> {
        warn!(
            ?event,
            "finished update weather process cannot handle event"
        );
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::weather::update::status::UpdateSteps;
    use claims::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_is_only_active_zone() {
        let mut location_statuses = MultiIndexLocationStatusMap::with_capacity(3);
        location_statuses.insert(LocationStatus::new(LocationZoneCode::new("foo")));
        location_statuses.insert(LocationStatus {
            zone: LocationZoneCode::new("bar"),
            status: LocationUpdateStatus::Succeeded,
        });
        location_statuses.insert(LocationStatus {
            zone: LocationZoneCode::new("zed"),
            status: LocationUpdateStatus::Succeeded,
        });

        let status = WeatherUpdateStatus { location_statuses, alerts_reviewed: false };

        assert!(status.is_only_active_zone(&LocationZoneCode::new("foo")));
        assert!(!status.is_only_active_zone(&LocationZoneCode::new("bar")));
        assert!(!status.is_only_active_zone(&LocationZoneCode::new("zed")));
        assert!(
            !status.is_only_active_zone(&LocationZoneCode::new("otis")),
            "otis is not a zone"
        );
    }

    #[test]
    fn test_weather_update_status_advance() {
        once_cell::sync::Lazy::force(&crate::setup_tracing::TEST_TRACING);
        let main_span = info_span!("test_weather_update_status_advance");
        let _ = main_span.enter();

        let otis = LocationZoneCode::new("otis");
        let stella = LocationZoneCode::new("stella");
        let neo = LocationZoneCode::new("neo");

        let mut status = WeatherUpdateStatus::new(vec![otis.clone(), stella.clone(), neo.clone()]);
        status.alerts_reviewed = true;
        assert_eq!(
            status.active_zones(),
            maplit::hashset! { otis.clone(), stella.clone(), neo.clone() }
        );

        let state_otis = status.advance_zone_step(&otis, UpdateStep::Alert);
        assert_eq!(state_otis, UpdateWeatherStateDiscriminants::Active);
        assert_eq!(
            assert_some!(status.status_for(&otis)),
            LocationUpdateStatus::InProgress(UpdateSteps::default() | UpdateStep::Alert)
        );
        assert_eq!(
            status.active_zones(),
            maplit::hashset! { otis.clone(), stella.clone(), neo.clone() }
        );

        let state_otis = status.advance_zone_step(&otis, UpdateStep::Observation);
        assert_eq!(state_otis, UpdateWeatherStateDiscriminants::Active);
        assert_eq!(
            assert_some!(status.status_for(&otis)),
            LocationUpdateStatus::InProgress(
                UpdateSteps::default() | UpdateStep::Alert | UpdateStep::Observation
            )
        );
        assert_eq!(
            status.active_zones(),
            maplit::hashset! { otis.clone(), stella.clone(), neo.clone() }
        );

        info!("DMR: start final step...");
        let state = status.advance_zone_step(&otis, UpdateStep::Forecast);
        assert_eq!(state, UpdateWeatherStateDiscriminants::Active);
        assert_eq!(
            assert_some!(status.status_for(&otis)),
            LocationUpdateStatus::Succeeded
        );
        assert_eq!(
            status.active_zones(),
            maplit::hashset! { stella.clone(), neo.clone() }
        );

        assert_eq!(status.succeeded_zones(), maplit::hashset! { otis.clone() });
        assert_eq!(
            status.active_zones(),
            maplit::hashset! { stella.clone(), neo.clone() }
        );

        // bring to completion
        status.advance_zone_step(&stella, UpdateStep::Forecast);
        status.advance_zone_step(&stella, UpdateStep::Observation);
        status.advance_zone_step(&stella, UpdateStep::Alert);
        assert_eq!(
            status.succeeded_zones(),
            maplit::hashset! { otis.clone(), stella.clone() }
        );

        status.advance_zone_step(&neo, UpdateStep::Alert);
        status.advance_zone_step(&neo, UpdateStep::Forecast);
        let state = status.advance_zone_step(&neo, UpdateStep::Observation);
        assert_eq!(state, UpdateWeatherStateDiscriminants::Finished);
        assert_eq!(
            status.succeeded_zones(),
            maplit::hashset! { otis.clone(), stella.clone(), neo.clone()}
        )
    }
}
