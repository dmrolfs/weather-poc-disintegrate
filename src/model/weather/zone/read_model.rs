use crate::model::weather::zone::LocationZoneError;
use crate::model::weather::LocationZoneEvent;
use crate::model::{LocationZoneCode, WeatherAlert, WeatherFrame, ZoneForecast};
use crate::postgres::{TableColumn, TableName, LAST_UPDATED_AT_COL};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use disintegrate::{query, EventListener, PersistedEvent, StreamQuery};
use once_cell::sync::{Lazy, OnceCell};
use sql_query_builder as sql;
use sqlx::postgres::PgQueryResult;
use sqlx::types::Json;
use sqlx::{PgConnection, PgPool};
use std::clone::Clone;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct WeatherRepository {
    pool: PgPool,
}

pub const ZONE_WEATHER_VIEW: &str = "zone_weather";
pub static ZONE_WEATHER_TABLE: Lazy<TableName> =
    Lazy::new(|| TableName::from_str(ZONE_WEATHER_VIEW).unwrap());
static PRIMARY_KEY: Lazy<TableColumn> = Lazy::new(|| TableColumn::new("zone").unwrap());
static CURRENT_COL: Lazy<TableColumn> = Lazy::new(|| TableColumn::new("current").unwrap());
static FORECAST_COL: Lazy<TableColumn> = Lazy::new(|| TableColumn::new("forecast").unwrap());
static ALERT_COL: Lazy<TableColumn> = Lazy::new(|| TableColumn::new("alert").unwrap());

static COLUMNS: Lazy<[TableColumn; 5]> = Lazy::new(|| {
    [
        PRIMARY_KEY.clone(),
        CURRENT_COL.clone(),
        FORECAST_COL.clone(),
        ALERT_COL.clone(),
        LAST_UPDATED_AT_COL.clone(),
    ]
});
static COLUMNS_REP: Lazy<String> = Lazy::new(|| COLUMNS.join(", "));
static VALUES_REP: Lazy<String> = Lazy::new(|| {
    let values = (1..=COLUMNS.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");

    format!("( {values} )")
});

impl WeatherRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn weather_by_zone(
        &self, zone: &LocationZoneCode,
    ) -> Result<Option<ZoneWeather>, sqlx::Error> {
        static WEATHER_BY_ZONE_SQL: OnceCell<String> = OnceCell::new();
        let sql = WEATHER_BY_ZONE_SQL.get_or_init(|| {
            sql::Select::new()
                .select(&COLUMNS_REP)
                .from(&ZONE_WEATHER_TABLE)
                .where_clause(format!("{} = $1", PRIMARY_KEY.as_str()).as_str())
                .to_string()
        });

        sqlx::query_as(sql).bind(zone).fetch_optional(&self.pool).await
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ZoneWeather {
    pub zone: LocationZoneCode,
    pub current: Option<WeatherFrame>,
    pub forecast: Option<ZoneForecast>,
    pub alert: Option<WeatherAlert>,
    pub last_updated_at: DateTime<Utc>,
}

impl<'r, R> sqlx::FromRow<'r, R> for ZoneWeather
where
    R: sqlx::Row,
    Json<WeatherFrame>:
        sqlx::Decode<'r, <R as sqlx::Row>::Database> + sqlx::Type<<R as sqlx::Row>::Database>,
    Json<ZoneForecast>:
        sqlx::Decode<'r, <R as sqlx::Row>::Database> + sqlx::Type<<R as sqlx::Row>::Database>,
    Json<WeatherAlert>:
        sqlx::Decode<'r, <R as sqlx::Row>::Database> + sqlx::Type<<R as sqlx::Row>::Database>,
    String: sqlx::Decode<'r, <R as sqlx::Row>::Database> + sqlx::Type<<R as sqlx::Row>::Database>,
    DateTime<Utc>:
        sqlx::Decode<'r, <R as sqlx::Row>::Database> + sqlx::Type<<R as sqlx::Row>::Database>,
{
    #[instrument(level = "debug", skip(row), ret, err)]
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        let zone = row.try_get(PRIMARY_KEY.clone())?;

        let current_json = row.try_get::<Option<Json<_>>, _>(CURRENT_COL.clone());
        debug!("DMR: current_json={current_json:?}");
        let current = current_json?.map(|c| c.0);
        // let current = serde_json::from_value(current_json).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

        let forecast_json = row.try_get::<Option<Json<_>>, _>(FORECAST_COL.clone());
        debug!("DMR: forecast_json={forecast_json:?}");
        let forecast = forecast_json?.map(|f| f.0);
        // let forecast = serde_json::from_value(forecast_json).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

        let alert_json = row.try_get::<Option<Json<_>>, _>(ALERT_COL.clone());
        debug!("DMR: alert_json={alert_json:?}");
        let alert = alert_json?.map(|a| a.0);
        // let alert = serde_json::from_value(alert_json).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

        let last_updated_at = row.try_get(LAST_UPDATED_AT_COL.clone())?;

        Ok(Self { zone, current, forecast, alert, last_updated_at })
    }
}

#[derive(Debug)]
pub struct ZoneWeatherProjection {
    query: StreamQuery<LocationZoneEvent>,
    pool: PgPool,
}

impl ZoneWeatherProjection {
    pub async fn new(pool: PgPool) -> Result<Self, sqlx::Error> {
        static CREATE_TABLE_DDL: OnceCell<String> = OnceCell::new();
        let sql = CREATE_TABLE_DDL.get_or_init(|| {
            format!(
                r#"
                CREATE TABLE IF NOT EXISTS {table} (
                    {primary_key} TEXT PRIMARY KEY,
                    {current} JSONB,
                    {forecast} JSONB,
                    {alert} JSONB,
                    {last_updated_at} TIMESTAMPTZ NULL DEFAULT clock_timestamp()
                )"#,
                table = ZONE_WEATHER_TABLE.as_str(),
                primary_key = PRIMARY_KEY.as_str(),
                current = CURRENT_COL.as_str(),
                forecast = FORECAST_COL.as_str(),
                alert = ALERT_COL.as_str(),
                last_updated_at = LAST_UPDATED_AT_COL.as_str(),
            )
        });

        sqlx::query(sql).execute(&pool).await?;
        Ok(Self { query: query(None), pool })
    }
}

#[async_trait]
impl EventListener<LocationZoneEvent> for ZoneWeatherProjection {
    type Error = LocationZoneError;

    fn id(&self) -> &'static str {
        &ZONE_WEATHER_TABLE
    }

    fn query(&self) -> &StreamQuery<LocationZoneEvent> {
        &self.query
    }

    #[allow(clippy::blocks_in_conditions)]
    #[instrument(level = "debug", skip(self), err)]
    async fn handle(&self, event: PersistedEvent<LocationZoneEvent>) -> Result<(), Self::Error> {
        let mut tx = sqlx::Acquire::begin(&self.pool).await?;
        let result: PgQueryResult = match event.into_inner() {
            LocationZoneEvent::ObservationUpdated { zone, weather, .. } => {
                Self::update_or_insert_weather(zone, weather, &mut tx).await?
            },

            LocationZoneEvent::ForecastUpdated { zone, forecast, .. } => {
                Self::update_or_insert_forecast(zone, forecast, &mut tx).await?
            },

            LocationZoneEvent::AlertActivated { zone, alert, .. } => {
                Self::update_or_insert_alert(zone, Some(alert), &mut tx).await?
            },

            LocationZoneEvent::AlertDeactivated { zone, .. } => {
                Self::update_or_insert_alert(zone, None, &mut tx).await?
            },
        };

        let outcome = tx.commit().await;
        if let Err(ref error) = outcome {
            error!(
                "postgres projection failed to commit location zone event transaction: {error:?}"
            );
        }

        debug!("location zone projection postgres query result: {result:?}");
        outcome.map_err(|err| err.into())
    }
}

// static UPDATE_FORECAST_SQL: Lazy<sql::Update> = Lazy::new(|| {
//     sql::Update::new()
//         .update(&ZONE_WEATHER_TABLE)
//         .set("forecast = $2")
//         .where_clause(format!("{PRIMARY_KEY} = $1").as_str())
// });
//
// static UPDATE_ALERT_SQL: Lazy<sql::Update> = Lazy::new(|| {
//     sql::Update::new()
//         .update(&ZONE_WEATHER_TABLE)
//         .set("alert = $2")
//         .where_clause("zone = $1")
// });
//
// static UPDATE_CLEAR_ALERT_SQL: Lazy<sql::Update> = Lazy::new(|| {
//     sql::Update::new()
//         .update(&ZONE_WEATHER_TABLE)
//         .set("alert = NULL")
//         .where_clause("zone = $1")
// });

impl ZoneWeatherProjection {
    #[instrument(level = "debug", ret)]
    fn build_insert(update_clause: sql::Update) -> String {
        let conflict_clause = format!(
            "( {key} ) DO UPDATE {update_clause}",
            key = PRIMARY_KEY.as_str()
        );

        sql::Insert::new()
            .insert_into(
                format!(
                    "{table} ( {columns} )",
                    table = ZONE_WEATHER_TABLE.as_str(),
                    columns = COLUMNS_REP.as_str()
                )
                .as_str(),
            )
            .values(&VALUES_REP)
            .on_conflict(conflict_clause.as_str())
            .to_string()
    }

    #[instrument(level = "debug", skip(weather, tx), ret, err)]
    async fn update_or_insert_weather(
        zone: LocationZoneCode, weather: Arc<WeatherFrame>, tx: &mut PgConnection,
    ) -> Result<PgQueryResult, LocationZoneError> {
        static UPDATE_OR_INSERT_WEATHER_SQL: OnceCell<String> = OnceCell::new();
        let sql = UPDATE_OR_INSERT_WEATHER_SQL.get_or_init(|| {
            Self::build_insert(
                sql::Update::new()
                    .set("current = EXCLUDED.current, last_updated_at = EXCLUDED.last_updated_at"),
            )
        });

        debug!("sql: {sql}");

        sqlx::query(sql)
            .bind(zone) // zone
            .bind(Some(serde_json::to_value(weather)?)) // weather
            .bind(None::<serde_json::Value>) // forecast
            .bind(None::<serde_json::Value>) // alert
            .bind(Utc::now()) // last_updated_at
            .execute(tx)
            .await
            .map_err(|err| err.into())
    }

    #[instrument(level = "debug", skip(forecast, tx), ret, err)]
    async fn update_or_insert_forecast(
        zone: LocationZoneCode, forecast: Arc<ZoneForecast>, tx: &mut PgConnection,
    ) -> Result<PgQueryResult, LocationZoneError> {
        static UPDATE_OR_INSERT_FORECAST_SQL: OnceCell<String> = OnceCell::new();
        let sql =
            UPDATE_OR_INSERT_FORECAST_SQL.get_or_init(|| {
                Self::build_insert(sql::Update::new().set(
                    "forecast = EXCLUDED.forecast, last_updated_at = EXCLUDED.last_updated_at",
                ))
            });

        debug!("sql: {sql}");

        sqlx::query(sql)
            .bind(zone) // zone
            .bind(None::<serde_json::Value>) // current
            .bind(Some(serde_json::to_value(forecast)?)) // forecast
            .bind(None::<serde_json::Value>) // alert
            .bind(Utc::now()) // last_updated_at
            .execute(tx)
            .await
            .map_err(|err| err.into())
    }

    #[instrument(level = "debug", skip(alert, tx), ret, err)]
    async fn update_or_insert_alert(
        zone: LocationZoneCode, alert: Option<Arc<WeatherAlert>>, tx: &mut PgConnection,
    ) -> Result<PgQueryResult, LocationZoneError> {
        static UPDATE_OR_INSERT_ALERT_SQL: OnceCell<String> = OnceCell::new();
        let sql = UPDATE_OR_INSERT_ALERT_SQL.get_or_init(|| {
            Self::build_insert(
                sql::Update::new()
                    .set("alert = EXCLUDED.alert, last_updated_at = EXCLUDED.last_updated_at"),
            )
        });

        debug!("sql: {sql}");

        let query = sqlx::query(sql)
            .bind(zone) // zone
            .bind(None::<serde_json::Value>) // weather
            .bind(None::<serde_json::Value>); // forecast

        let alert_binding = alert.map(serde_json::to_value).transpose()?;
        query
            .bind(alert_binding)
            .bind(Utc::now())
            .execute(tx)
            .await
            .map_err(|err| err.into())
    }
}
