use crate::model::registrar::support::RegistrarSupport;
use crate::model::registrar::{
    MonitoredLocationZonesRef, RegistrarDecisionMakerRef, RegistrarEventSerde,
};
use crate::model::weather::update::{
    UpdateWeatherRepository, UpdateWeatherServices, UpdateWeatherServicesRef, UpdateWeatherSupport,
};
use crate::model::weather::zone::{LocationZoneSupport, WeatherRepository};
use crate::model::weather::{WeatherDecisionMakerRef, WeatherEventSerde, WeatherSupport};
use crate::server::api_errors::ApiBootstrapError;
use crate::server::get_connection_pool;
use crate::services::noaa::{NoaaWeatherApi, NoaaWeatherServices};
use crate::Settings;
use axum::extract::FromRef;
use disintegrate_postgres::PgEventStore;
use sqlx::PgPool;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use tokio_util::task::TaskTracker;
use url::Url;

#[derive(Clone)]
pub struct AppState {
    pub registrar_support: RegistrarSupport,
    pub weather_support: WeatherSupport,
    pub location_zone_support: LocationZoneSupport,
    pub update_weather_support: UpdateWeatherSupport,
    pub db_pool: PgPool,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState").finish()
    }
}

impl FromRef<AppState> for RegistrarDecisionMakerRef {
    fn from_ref(app: &AppState) -> Self {
        app.registrar_support.decision_maker.clone()
    }
}

impl FromRef<AppState> for WeatherDecisionMakerRef {
    fn from_ref(app: &AppState) -> Self {
        app.weather_support.decision_maker.clone()
    }
}

impl FromRef<AppState> for WeatherRepository {
    fn from_ref(app: &AppState) -> Self {
        app.location_zone_support.weather_repository.clone()
    }
}

impl FromRef<AppState> for MonitoredLocationZonesRef {
    fn from_ref(app: &AppState) -> Self {
        app.registrar_support.monitored.clone()
    }
}

impl FromRef<AppState> for UpdateWeatherRepository {
    fn from_ref(app: &AppState) -> Self {
        app.update_weather_support.history_repository.clone()
    }
}

impl FromRef<AppState> for UpdateWeatherServicesRef {
    fn from_ref(app: &AppState) -> Self {
        app.update_weather_support.services.clone()
    }
}

// impl FromRef<AppState> for UpdateWeatherHistoryProjection {
//     fn from_ref(app: &AppState) -> Self {
//         app.update_weather_support.update_history_projection.clone()
//     }
// }

impl FromRef<AppState> for PgPool {
    fn from_ref(app: &AppState) -> Self {
        app.db_pool.clone()
    }
}

impl AppState {
    #[instrument(level = "debug", skip(settings), err)]
    pub async fn new(
        settings: &Settings, task_tracker: &TaskTracker,
    ) -> Result<AppState, ApiBootstrapError> {
        info!(?settings, "creating application state");
        let db_pool = get_connection_pool(&settings.database);
        warn!("DMR - AAA");

        //todo: WORK TO CONSOLIDATE IN MOD SUPPORTS
        // -- Weather Core --
        let registrar_event_store =
            PgEventStore::new(db_pool.clone(), RegistrarEventSerde::default()).await?;
        warn!("DMR - BBB");
        let weather_event_store =
            PgEventStore::new(db_pool.clone(), WeatherEventSerde::default()).await?;
        warn!("DMR - CCC");

        let user_agent = reqwest::header::HeaderValue::from_str("(here.com, contact@example.com)")?;
        warn!("DMR - DDD");
        let base_url = Url::from_str("https://api.weather.gov")?;
        warn!("DMR - EEE");
        let noaa_api = NoaaWeatherApi::new(base_url, user_agent)?;
        warn!("DMR - FFF");
        let noaa = NoaaWeatherServices::Noaa(noaa_api);
        let update_weather_services = Arc::new(UpdateWeatherServices::new(noaa.clone()));
        warn!("DMR - GGG");
        // -- Weather Core --

        // -- Registrar --
        let registrar_support = RegistrarSupport::new(
            registrar_event_store,
            update_weather_services.clone(),
            task_tracker,
        )
        .await?;
        warn!("DMR - HHH");
        // -- Registrar --

        // -- Weather --
        let weather_support = WeatherSupport::new(weather_event_store.clone()).await?;
        warn!("DMR - III");
        // -- Weather --

        // -- Location Zone --
        let location_zone_support = LocationZoneSupport::new(
            db_pool.clone(),
            weather_event_store.clone(),
            noaa.clone(),
            task_tracker,
        )
        .await?;
        warn!("DMR - JJJ");
        // -- Location Zone --

        // -- Update Weather --
        let update_weather_support = UpdateWeatherSupport::new(
            db_pool.clone(),
            weather_event_store,
            update_weather_services,
            task_tracker,
        )
        .await?;
        warn!("DMR - KKK");
        // -- Update WeIIIather --

        // let journal_storage_config =
        //     settings::storage_config_from(&settings.database, &settings.zone);
        // let journal_storage_provider =
        //     PostgresStorageProvider::connect(journal_storage_config, &system).await?;
        // let journal_storage = journal_storage_provider
        //     .processor_source()
        //     .ok_or_else(|| anyhow!("no journal processor storage!"))
        //     .map_err(coerce_cqrs::postgres::PostgresStorageError::Storage)?;
        //
        // let system = system.to_persistent(Persistence::from(journal_storage_provider));

        // -- registrar
        // let registrar_support = Registrar::initialize_aggregate_support(
        //     journal_storage.clone(),
        //     RegistrarService::full(),
        //     settings,
        //     &system,
        // )
        // .await?;

        // -- location zone

        // let location_zone_support = LocationZone::initialize_aggregate_support(
        //     journal_storage.clone(),
        //     noaa,
        //     settings,
        //     &system,
        // )
        // .await?;

        // -- update locations
        // let update_locations_support = UpdateLocations::initialize_aggregate_support(
        //     journal_storage.clone(),
        //     journal_storage.clone(),
        //     settings,
        //     &system,
        // )
        // .await?;

        Ok(AppState {
            registrar_support,
            weather_support,
            location_zone_support,
            update_weather_support,
            db_pool,
        })
    }
}
