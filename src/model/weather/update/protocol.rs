use super::state::{UpdateWeather, UpdateWeatherId, UpdateWeatherState};
use super::{UpdateWeatherError, UpdateWeatherServicesRef};
use crate::model::weather::zone::LocationZoneError;
use crate::model::weather::{zone, WeatherDecisionMakerRef, WeatherEvent};
use crate::model::{LocationZoneCode, WeatherAlert};
use crate::services::noaa::AlertApi;
use disintegrate::Decision;
use std::collections::{HashMap, HashSet};
use std::fmt;
use tagid::Entity;
use tracing_futures::Instrument;

// #[derive(Debug, Display, Clone, PartialEq, Eq, Event, Serialize, Deserialize)]
// pub enum UpdateWeatherEvent {
//     Started(Vec<LocationZoneCode>),
//     LocationUpdated(LocationZoneCode, LocationUpdateStatus),
//     AlertsUpdated,
//     Completed,
//     Failed,
// }

#[derive(Debug, PartialEq, Eq)]
pub struct NoteAlertsReviewed(pub UpdateWeatherId);

impl Decision for NoteAlertsReviewed {
    type Event = WeatherEvent;
    type StateQuery = UpdateWeather;
    type Error = UpdateWeatherError;

    fn state_query(&self) -> Self::StateQuery {
        UpdateWeather::new(self.0.clone())
    }

    fn process(&self, state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        use UpdateWeatherState as S;

        match &state.state {
            S::Active(_) => Ok(vec![WeatherEvent::AlertsReviewed {
                update_id: self.0.clone(),
            }]),
            S::Quiescent(_) => Err(UpdateWeatherError::NotStarted(
                self.0.clone(),
                tynm::type_name::<Self>(),
            )),
            S::Finished(_) => Err(UpdateWeatherError::Finished(
                self.0.clone(),
                tynm::type_name::<Self>(),
            )),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct NoteLocationUpdateFailure {
    pub update_id: UpdateWeatherId,
    pub zone: LocationZoneCode,
    pub cause: String,
}

impl Decision for NoteLocationUpdateFailure {
    type Event = WeatherEvent;
    type StateQuery = UpdateWeather;
    type Error = UpdateWeatherError;

    fn state_query(&self) -> Self::StateQuery {
        UpdateWeather::new(self.update_id.clone())
    }

    #[instrument(level = "debug", ret, err)]
    fn process(&self, state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        use UpdateWeatherState as S;

        match &state.state {
            S::Active(_) => Ok(vec![WeatherEvent::UpdateLocationFailed {
                update_id: self.update_id.clone(),
                zone: self.zone.clone(),
                cause: self.cause.clone(),
            }]),
            S::Quiescent(_) => Err(UpdateWeatherError::NotStarted(
                self.update_id.clone(),
                tynm::type_name::<Self>(),
            )),
            S::Finished(_) => Err(UpdateWeatherError::Finished(
                self.update_id.clone(),
                tynm::type_name::<Self>(),
            )),
        }
    }
}

pub struct StartUpdate {
    update_id: UpdateWeatherId,
    zones: Vec<LocationZoneCode>,
    weather_dm: WeatherDecisionMakerRef,
    services: UpdateWeatherServicesRef,
}

impl fmt::Debug for StartUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StartUpdate")
            .field("update_id", &self.update_id)
            .field("zones", &self.zones)
            .field("services", &self.services)
            .finish()
    }
}

impl PartialEq for StartUpdate {
    fn eq(&self, other: &Self) -> bool {
        self.update_id == other.update_id && self.zones == other.zones
    }
}

impl Eq for StartUpdate {}

impl StartUpdate {
    pub fn for_zones(
        zones: Vec<LocationZoneCode>, weather_dm: WeatherDecisionMakerRef,
        services: UpdateWeatherServicesRef,
    ) -> Result<Self, UpdateWeatherError> {
        Self::new(UpdateWeather::next_id(), zones, weather_dm, services)
    }

    pub fn new(
        update_id: UpdateWeatherId, zones: Vec<LocationZoneCode>,
        weather_dm: WeatherDecisionMakerRef, services: UpdateWeatherServicesRef,
    ) -> Result<Self, UpdateWeatherError> {
        if zones.is_empty() {
            return Err(UpdateWeatherError::NoLocations);
        }

        Ok(Self { update_id, zones, weather_dm, services })
    }
}

impl Decision for StartUpdate {
    type Event = WeatherEvent;
    type StateQuery = UpdateWeather;
    type Error = UpdateWeatherError;

    fn state_query(&self) -> Self::StateQuery {
        UpdateWeather::new(self.update_id.clone())
    }

    #[instrument(level = "debug", name = "UpdateWeather::process", ret, err)]
    fn process(&self, state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        match state.state {
            UpdateWeatherState::Quiescent(_) => {
                self.do_start_update();
                Ok(vec![WeatherEvent::UpdateStarted {
                    update_id: self.update_id.clone(),
                    zones: self.zones.clone(),
                }])
            },
            _ => Err(UpdateWeatherError::AlreadyStarted(
                self.update_id.clone(),
                self.zones.clone(),
            )),
        }
    }
}

impl StartUpdate {
    fn do_start_update(&self) {
        self.do_spawn_zone_updates();
        self.do_spawn_alerts();
    }

    fn do_spawn_zone_updates(&self) {
        self.zones.iter().cloned().for_each(|z| {
            let update_id_o = self.update_id.clone();
            let update_id_o_i = self.update_id.clone();
            let update_id_f = self.update_id.clone();
            let update_id_f_i = self.update_id.clone();
            let weather_dm_o = self.weather_dm.clone();
            let weather_dm_f = self.weather_dm.clone();
            let zone_o = z.clone();
            let zone_o_i = z.clone();
            let zone_f = z.clone();
            let zone_f_i = z.clone();
            tokio::spawn(
                async move { zone::observe(update_id_o, zone_o, weather_dm_o).await }.instrument(
                    debug_span!(
                        "observe location zone weather",
                        update_id=%update_id_o_i,zone=%zone_o_i
                    ),
                ),
            );

            tokio::spawn(
                async move { zone::forecast(update_id_f, zone_f, weather_dm_f).await }.instrument(
                    debug_span!(
                        "forecast location zone weather",
                        update_id=%update_id_f_i, zone=%zone_f_i
                    ),
                ),
            );
        });
    }

    fn do_spawn_alerts(&self) {
        let update_id = self.update_id.clone();
        let update_id_i = self.update_id.clone();
        let zones = self.zones.clone();
        let zones_i = self.zones.clone();
        let weather_dm = self.weather_dm.clone();
        let services = self.services.clone();

        tokio::spawn(
            async move {
                if let Err(error) =
                    do_update_zone_alerts(update_id, zones, weather_dm, services).await
                {
                    warn!(
                        ?error,
                        "failed to update location weather alerts -- ignoring"
                    );
                }
            }
            .instrument(debug_span!(
                "update location zone weather alerts",
                update_id=%update_id_i, zones=?zones_i,
            )),
        );
    }
}

#[instrument(level = "debug", skip(weather_dm), err)]
async fn do_update_zone_alerts(
    update_id: UpdateWeatherId, zones: Vec<LocationZoneCode>, weather_dm: WeatherDecisionMakerRef,
    services: UpdateWeatherServicesRef,
) -> Result<(), UpdateWeatherError> {
    let update_scope: HashSet<_> = zones.into_iter().collect();
    let mut alerted_zones = HashSet::with_capacity(update_scope.len());

    // -- zones with alerts
    let alerts = services.active_alerts().await?;
    let nr_alerts = alerts.len();
    let mut update_failures = HashMap::new();

    for alert in alerts {
        let affected: Vec<_> = alert
            .affected_zones
            .clone()
            .into_iter()
            .filter(|z| update_scope.contains(z))
            .collect();

        let (alerted, failures) =
            do_alert_affected_zones(update_id.clone(), affected, alert, weather_dm.clone()).await;
        alerted_zones.extend(alerted);
        update_failures.extend(failures);
    }

    // -- unaffected zones
    let unaffected_zones: Vec<_> = update_scope.difference(&alerted_zones).cloned().collect();
    info!(?alerted_zones, ?unaffected_zones, %nr_alerts, "DMR: finish alerting with affected notes...");
    let unaffected_failures =
        do_update_unaffected_zones(update_id.clone(), unaffected_zones, weather_dm.clone()).await;
    update_failures.extend(unaffected_failures);

    // -- note update failures
    do_note_alert_update_failures(update_id.clone(), update_failures, weather_dm.clone()).await?;

    // -- note alerts updated as far as they will be
    super::note_alerts_updated(update_id, weather_dm.clone()).await?;

    Ok(())
}

type ZoneUpdateFailures = HashMap<LocationZoneCode, LocationZoneError>;

#[instrument(level = "trace", skip(weather_dm), ret)]
async fn do_alert_affected_zones(
    update_id: UpdateWeatherId, affected: Vec<LocationZoneCode>, alert: WeatherAlert,
    weather_dm: WeatherDecisionMakerRef,
) -> (Vec<LocationZoneCode>, ZoneUpdateFailures) {
    let mut alerted = vec![];
    let mut failures = ZoneUpdateFailures::new();

    for zone in affected {
        alerted.push(zone.clone());
        if let Err(error) = zone::alert(
            update_id.clone(),
            zone.clone(),
            Some(alert.clone()),
            weather_dm.clone(),
        )
        .await
        {
            failures.insert(zone, LocationZoneError::Decision(Box::new(error)));
        }
    }

    (alerted, failures)
}

#[instrument(level = "trace", skip(weather_dm), ret)]
async fn do_update_unaffected_zones(
    update_id: UpdateWeatherId, unaffected: Vec<LocationZoneCode>,
    weather_dm: WeatherDecisionMakerRef,
) -> ZoneUpdateFailures {
    let mut failures = ZoneUpdateFailures::new();

    for zone in unaffected {
        if let Err(error) =
            zone::alert(update_id.clone(), zone.clone(), None, weather_dm.clone()).await
        {
            failures.insert(zone, LocationZoneError::Decision(Box::new(error)));
        }
    }

    failures
}

#[instrument(level = "debug", skip(weather_dm), err)]
async fn do_note_alert_update_failures(
    update_id: UpdateWeatherId, zone_failures: ZoneUpdateFailures,
    weather_dm: WeatherDecisionMakerRef,
) -> Result<(), UpdateWeatherError> {
    let mut errors = vec![];
    for (zone, failure) in zone_failures {
        if let Err(error) = super::note_zone_update_failure(
            update_id.clone(),
            zone,
            failure.into(),
            weather_dm.clone(),
        )
        .await
        {
            errors.push(error);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        warn!(
            ?errors,
            "failed to note {} location failures in `UpdateWeather` saga({update_id})",
            errors.len()
        );
        Err(errors.pop().unwrap())
    }
}
