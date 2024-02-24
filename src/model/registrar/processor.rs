// use crate::model::registrar::protocol::RegistrarEvent;
// use crate::model::registrar::RegistrarError;
// use async_trait::async_trait;
// use disintegrate::{query, EventListener, PersistedEvent, StreamQuery};
//
// #[derive(Debug)]
// pub struct RegistrarProcessor {
//     // services: RegistrarServicesRef,
//     // zone_dm: LocationZoneDecisionMakerRef,
//     query: StreamQuery<RegistrarEvent>,
// }
//
// impl RegistrarProcessor {
//     pub fn new() -> Self {
//         Self { query: query!(RegistrarEvent) }
//     }
// }
//
// #[async_trait]
// impl EventListener<RegistrarEvent> for RegistrarProcessor {
//     type Error = RegistrarError;
//
//     fn id(&self) -> &'static str {
//         "registrar_processor"
//     }
//
//     fn query(&self) -> &StreamQuery<RegistrarEvent> {
//         &self.query
//     }
//
//     async fn handle(&self, _event: PersistedEvent<RegistrarEvent>) -> Result<(), Self::Error> {
//         Ok(())
//     }
//     // async fn handle(&self, event: PersistedEvent<RegistrarEvent>) -> Result<(), Self::Error> {
//     //     // if let RegistrarEvent::ForecastZoneAdded(zone) = event.into_inner() {
//     //     //     self.services.initialize_forecast_zone(&zone, &self.zone_dm).await?;
//     //     // }
//     //
//     //     Ok(())
//     // }
// }
