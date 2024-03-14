#[macro_use]
extern crate tracing;

use claims::*;
use geojson::Feature;
use geojson::{FeatureCollection, GeoJson};
use iso8601_timestamp::Timestamp;
use pretty_assertions::assert_eq;
use settings_loader::settings_loader::SettingsLoader;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::Instrument;
use weather::zone::WeatherRepository;
use weather_disintegrate::model::weather::{self, WeatherEvent, WeatherEventSerde, WeatherSupport};
use weather_disintegrate::model::{LocationZoneCode, WeatherFrame, ZoneForecast};

#[test]
fn test_note_current_weather() -> anyhow::Result<()> {
    once_cell::sync::Lazy::force(&weather_disintegrate::setup_tracing::TEST_TRACING);
    let main_span = tracing::info_span!("test_note_current_weather");
    let _main_span_guard = main_span.enter();

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let actual_observation = assert_ok!(std::fs::read_to_string(
        "./tests/data/geojson-waz558-zone-observation-1.json"
    ));
    let actual_json: serde_json::Value = assert_ok!(serde_json::from_str(&actual_observation));
    assert!(actual_json.is_object());
    let actual_geojson: GeoJson = assert_ok!(actual_observation.parse());
    let actual_features = assert_ok!(FeatureCollection::try_from(actual_geojson));

    let update_id = weather::update::next_id();
    let zone = LocationZoneCode::random();
    let mut observation: WeatherFrame = actual_features.into();
    let time_rep = observation.timestamp.format();
    observation.timestamp = assert_some!(Timestamp::parse(time_rep.as_ref()));
    let command = weather::zone::protocol::NoteObservation::new(
        zone.clone(),
        update_id.clone(),
        observation.clone(),
    );

    tokio_test::block_on(
        async move {
            let options = weather_disintegrate::CliOptions {
                config: Some("./resources/application.yaml".into()),
                secrets: Some("./resources/secrets.yaml".into()),
                environment: Some("local".into()),
                ..Default::default()
            };

            let settings = assert_ok!(weather_disintegrate::Settings::load(&options));
            let pool = weather_disintegrate::server::get_connection_pool(&settings.database);
            let es = assert_ok!(
                disintegrate_postgres::PgEventStore::new(
                    pool.clone(),
                    WeatherEventSerde::default()
                )
                .await
            );
            let support = assert_ok!(WeatherSupport::new(es.clone()).await);
            let weather_dm = support.decision_maker.clone();

            let mut tasks: tokio::task::JoinSet<anyhow::Result<()>> = tokio::task::JoinSet::new();
            let pool_cmd = pool.clone();
            let zone_cmd = zone.clone();
            let observation_cmd = observation.clone();

            let events = assert_ok!(weather_dm.make(command).await);
            let events: Vec<_> = events.into_iter().map(|pe| pe.into_inner()).collect();
            assert!(!events.is_empty());
            assert_eq!(
                events,
                vec![WeatherEvent::ObservationUpdated {
                    zone: zone_cmd,
                    update_id: update_id.clone(),
                    weather: Arc::new(observation_cmd),
                }]
            );

            let weather_projection =
                assert_ok!(weather::zone::read_model::ZoneWeatherProjection::new(pool_cmd).await);

            let listener_config =
                disintegrate_postgres::PgEventListenerConfig::poller(Duration::from_millis(50));

            tasks.spawn(async move {
                assert_ok!(
                    disintegrate_postgres::PgEventListener::builder(es)
                        .register_listener(weather_projection, listener_config)
                        .start_with_shutdown(weather_disintegrate::shutdown())
                        .await
                );
                COUNTER.fetch_add(1, Ordering::Relaxed);
                Ok(())
            });

            tasks.spawn(
                async {
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    COUNTER.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
                .instrument(info_span!("SLEEP_TASK")),
            );

            tasks.join_next().await;
            assert_eq!(COUNTER.load(Ordering::Relaxed), 1);

            let weather_repository = WeatherRepository::new(pool.clone());
            let actual = assert_some!(assert_ok!(weather_repository.weather_by_zone(&zone).await));
            assert_eq!(actual.zone, zone);
            assert_eq!(assert_some!(actual.current), observation);
        }
        .instrument(info_span!("ASYNC_BLOCK")),
    );

    Ok(())
}
#[test]
fn test_note_weather_forecast() -> anyhow::Result<()> {
    once_cell::sync::Lazy::force(&weather_disintegrate::setup_tracing::TEST_TRACING);
    let main_span = tracing::info_span!("test_note_weather_forecast");
    let _main_span_guard = main_span.enter();

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let actual_forecast = assert_ok!(std::fs::read_to_string(
        "./tests/data/geojson-waz558-zone-forecast-1.json"
    ));
    let actual_json: serde_json::Value = assert_ok!(serde_json::from_str(&actual_forecast));
    assert!(actual_json.is_object());
    let actual_geojson: GeoJson = assert_ok!(actual_forecast.parse());
    let actual_feature = assert_ok!(Feature::try_from(actual_geojson));

    let update_id = weather::update::next_id();
    let zone = LocationZoneCode::random();
    let forecast: ZoneForecast = assert_ok!(actual_feature.try_into());
    let command = weather::zone::protocol::NoteForecast::new(
        zone.clone(),
        update_id.clone(),
        forecast.clone(),
    );

    tokio_test::block_on(
        async move {
            let options = weather_disintegrate::CliOptions {
                config: Some("./resources/application.yaml".into()),
                secrets: Some("./resources/secrets.yaml".into()),
                environment: Some("local".into()),
                ..Default::default()
            };

            let settings = assert_ok!(weather_disintegrate::Settings::load(&options));
            let pool = weather_disintegrate::server::get_connection_pool(&settings.database);
            let es = assert_ok!(
                disintegrate_postgres::PgEventStore::new(
                    pool.clone(),
                    WeatherEventSerde::default()
                )
                .await
            );
            let support = assert_ok!(WeatherSupport::new(es.clone()).await);
            let weather_dm = support.decision_maker.clone();

            let mut tasks: tokio::task::JoinSet<anyhow::Result<()>> = tokio::task::JoinSet::new();
            let pool_cmd = pool.clone();
            let zone_cmd = zone.clone();
            let forecast_cmd = forecast.clone();

            let events = assert_ok!(weather_dm.make(command).await);
            let events: Vec<_> = events.into_iter().map(|pe| pe.into_inner()).collect();
            assert!(!events.is_empty());
            assert_eq!(
                events,
                vec![WeatherEvent::ForecastUpdated {
                    zone: zone_cmd,
                    update_id: update_id.clone(),
                    forecast: Arc::new(forecast_cmd),
                }]
            );

            let weather_projection =
                assert_ok!(weather::zone::read_model::ZoneWeatherProjection::new(pool_cmd).await);

            let listener_config =
                disintegrate_postgres::PgEventListenerConfig::poller(Duration::from_millis(50));

            tasks.spawn(async move {
                assert_ok!(
                    disintegrate_postgres::PgEventListener::builder(es)
                        .register_listener(weather_projection, listener_config)
                        .start_with_shutdown(weather_disintegrate::shutdown())
                        .await
                );
                COUNTER.fetch_add(1, Ordering::Relaxed);
                Ok(())
            });

            tasks.spawn(
                async {
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    COUNTER.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
                .instrument(info_span!("SLEEP_TASK")),
            );

            tasks.join_next().await;
            assert_eq!(COUNTER.load(Ordering::Relaxed), 1);

            let weather_repository = WeatherRepository::new(pool.clone());
            let actual = assert_some!(assert_ok!(weather_repository.weather_by_zone(&zone).await));
            assert_eq!(actual.zone, zone);
            assert_eq!(assert_some!(actual.forecast), forecast);
        }
        .instrument(info_span!("ASYNC_BLOCK")),
    );

    Ok(())
}
