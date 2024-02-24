use crate::model::registrar::protocol::RegistrarEvent;
use crate::model::registrar::RegistrarError;
use crate::model::LocationZoneCode;
use async_trait::async_trait;
use disintegrate::{query, EventListener, PersistedEvent, StreamQuery};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitoredLocationZonesView(HashSet<LocationZoneCode>);

impl std::ops::Deref for MonitoredLocationZonesView {
    type Target = HashSet<LocationZoneCode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type MonitoredLocationZonesRef = Arc<MonitoredLocationZones>;

//todo: move projection into database!

#[derive(Debug, Clone)]
pub struct MonitoredLocationZones {
    zones: Arc<Mutex<HashSet<LocationZoneCode>>>,
    query: StreamQuery<RegistrarEvent>,
}

impl Default for MonitoredLocationZones {
    fn default() -> Self {
        Self {
            zones: Default::default(),
            query: query!(RegistrarEvent),
        }
    }
}

impl MonitoredLocationZones {
    pub fn monitored(&self) -> MonitoredLocationZonesView {
        MonitoredLocationZonesView(self.zones.lock().unwrap().clone())
    }
}

#[async_trait]
impl EventListener<RegistrarEvent> for MonitoredLocationZones {
    type Error = RegistrarError;

    fn id(&self) -> &'static str {
        "monitored_location_zones"
    }

    fn query(&self) -> &StreamQuery<RegistrarEvent> {
        &self.query
    }

    async fn handle(
        &self, persisted_event: PersistedEvent<RegistrarEvent>,
    ) -> Result<(), Self::Error> {
        use RegistrarEvent as E;

        let mut my_zones = self.zones.lock().unwrap();
        match persisted_event.into_inner() {
            E::ForecastZoneAdded { zone } => {
                my_zones.insert(zone);
            },
            E::ForecastZoneRemoved { zone } => {
                my_zones.remove(&zone);
            },
            E::AllForecastZonesRemoved => {
                my_zones.clear();
            },
        }

        Ok(())
    }
}
