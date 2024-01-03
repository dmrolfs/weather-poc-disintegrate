#[macro_use]
extern crate tracing;

use claim::*;
use geojson::{FeatureCollection, GeoJson};
use pretty_assertions::assert_eq;
use std::collections::HashSet;
use weather_coerce::model::WeatherAlert;

#[test]
fn test_active_alert_deser() -> anyhow::Result<()> {
    once_cell::sync::Lazy::force(&coerce_cqrs_test::setup_tracing::TEST_TRACING);
    let main_span = tracing::info_span!("test_active_alert_deser");
    let _main_span_guard = main_span.enter();

    info!("current directory:{:?}", std::env::current_dir());
    let active_alerts = assert_ok!(std::fs::read_to_string(
        "./tests/data/geojson-active_alerts-1.json"
    ));

    let actual_json: serde_json::Value = assert_ok!(serde_json::from_str(&active_alerts));
    assert_eq!(actual_json.is_object(), true);

    let actual_geo: GeoJson = assert_ok!(active_alerts.parse());
    let actual_features = assert_ok!(FeatureCollection::try_from(actual_geo));
    let actual_alerts = actual_features.features.into_iter().map(|f| {
        info!(feature=?f, "weather feature.");
        assert_ok!(WeatherAlert::try_from(f))
    });
    assert_eq!(actual_alerts.len(), 326);
    let mut affected = HashSet::new();
    for a in actual_alerts.clone() {
        // info!("alert error: ")
        // let a: WeatherAlert = assert_ok!(a);
        affected.extend(a.affected_zones);
    }
    assert_eq!(affected.len(), 399);
    info!("affected zones: {affected:?}");

    Ok(())
}
