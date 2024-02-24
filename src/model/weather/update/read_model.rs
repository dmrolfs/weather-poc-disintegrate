use crate::model::weather::update::state::{UpdateWeatherStateDiscriminants, WeatherUpdateStatus};
use crate::model::weather::update::{UpdateWeatherError, UpdateWeatherId};
use crate::model::weather::WeatherEvent;
use crate::model::LocationZoneCode;
use crate::postgres::{TableColumn, TableName, LAST_UPDATED_AT_COL};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use disintegrate::{query, EventListener, PersistedEvent, StreamQuery};
use once_cell::sync::{Lazy, OnceCell};
use sql_query_builder as sql;
use sqlx::postgres::PgQueryResult;
use sqlx::{ColumnIndex, FromRow, PgConnection, PgPool};
use std::clone::Clone;
use std::str::FromStr;

pub const UPDATE_WEATHER_HISTORY: &str = "update_weather_history";

#[derive(Debug, PartialEq, ToSchema, Serialize)]
pub struct UpdateWeatherStatusView {
    pub update_id: UpdateWeatherId,
    pub state: UpdateWeatherStateDiscriminants,
    pub update_statuses: WeatherUpdateStatus,
    pub last_updated_at: DateTime<Utc>,
}

impl<'r, R> FromRow<'r, R> for UpdateWeatherStatusView
where
    R: sqlx::Row,
    usize: ColumnIndex<R>,
    String: sqlx::Decode<'r, <R as sqlx::Row>::Database> + sqlx::Type<<R as sqlx::Row>::Database>,
    UpdateWeatherId:
        sqlx::Decode<'r, <R as sqlx::Row>::Database> + sqlx::Type<<R as sqlx::Row>::Database>,
    UpdateWeatherStateDiscriminants:
        sqlx::Decode<'r, <R as sqlx::Row>::Database> + sqlx::Type<<R as sqlx::Row>::Database>,
    DateTime<Utc>:
        sqlx::Decode<'r, <R as sqlx::Row>::Database> + sqlx::Type<<R as sqlx::Row>::Database>,
{
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        let update_id = row.try_get(PRIMARY_KEY.clone())?;

        let state = row.try_get(STATE_COL.clone())?;

        let ls_rep: String = row.try_get(UPDATE_STATUSES_COL.clone())?;
        let status =
            serde_json::from_str(&ls_rep).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

        let last_updated_at = row.try_get(LAST_UPDATED_AT_COL.clone())?;

        Ok(Self {
            update_id,
            state,
            update_statuses: status,
            last_updated_at,
        })
    }
}

#[derive(Debug, Clone)]
pub struct UpdateWeatherRepository {
    pool: PgPool,
}

pub static UPDATE_WEATHER_HISTORY_TABLE: Lazy<TableName> =
    Lazy::new(|| TableName::from_str(UPDATE_WEATHER_HISTORY).unwrap());
static PRIMARY_KEY: Lazy<TableColumn> = Lazy::new(|| TableColumn::from_str("update_id").unwrap());
static STATE_COL: Lazy<TableColumn> = Lazy::new(|| TableColumn::from_str("state").unwrap());
static UPDATE_STATUSES_COL: Lazy<TableColumn> =
    Lazy::new(|| TableColumn::from_str("update_statuses").unwrap());
static COLUMNS: Lazy<[TableColumn; 4]> = Lazy::new(|| {
    [
        PRIMARY_KEY.clone(),
        STATE_COL.clone(),
        UPDATE_STATUSES_COL.clone(),
        LAST_UPDATED_AT_COL.clone(),
    ]
});
static COLUMNS_REP: Lazy<String> = Lazy::new(|| COLUMNS.join(", "));
static VALUES_REP: Lazy<String> = Lazy::new(|| {
    let values = (1..=COLUMNS_REP.len())
        .map(|i| format!("${i}"))
        .collect::<Vec<_>>()
        .join(", ");

    format!("( {values} )")
});

impl UpdateWeatherRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn fetch_optional_update_status(
        &self, update_id: &UpdateWeatherId,
    ) -> Result<Option<UpdateWeatherStatusView>, UpdateWeatherError> {
        sqlx::query_as(
            r#"
            SELECT update_id, state, update_statuses, last_updated_at
            FROM update_weather_history
            WHERE update_id = $1
            LIMIT 1
            "#,
        )
        .bind(update_id.clone())
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| err.into())

        // static UPDATE_STATUS_BY_ID_SQL: OnceCell<String> = OnceCell::new();
        // let sql = UPDATE_STATUS_BY_ID_SQL.get_or_init(|| {
        //     sql::Select::new()
        //         .select(&COLUMNS_REP)
        //         .from(&UPDATE_WEATHER_HISTORY_TABLE)
        //         .where_clause(format!("{} = $1", PRIMARY_KEY.as_str()).as_str())
        //         .to_string()
        // });
        //
        // sqlx::query_as(sql).bind(update_id).fetch_optional(&self.pool).await
    }
}

static STATUS_BY_ID_SQL: Lazy<String> = Lazy::new(|| {
    sql::Select::new()
        .select(COLUMNS_REP.as_str())
        .from(UPDATE_WEATHER_HISTORY_TABLE.as_str())
        .where_clause(format!("{} = $1", PRIMARY_KEY.as_str()).as_str())
        .to_string()
});

// async fn do_fetch_optional_view<'q, 'e, 'c, DB, E>(
// async fn do_fetch_optional_view<'e, DB, E>(
//     update_id: &UpdateWeatherId, executor: E,
// ) -> Result<Option<UpdateWeatherStatusView>, UpdateWeatherError>
// where
//     E: sqlx::Executor<'e, Database = DB>,
//     DB: sqlx::Database,
//     <DB as sqlx::database::HasArguments<'e>>::Arguments: sqlx::IntoArguments<'e, DB>,
//     // &'e str: sqlx::Encode<'e, DB> + sqlx::Decode<'e, DB> + sqlx::Type<DB>,
//     String: sqlx::Encode<'e, DB> + sqlx::Decode<'e, DB> + sqlx::Type<DB>,
//     DateTime<Utc>: sqlx::Encode<'e, DB> + sqlx::Decode<'e, DB> + sqlx::Type<DB>,
//     usize: ColumnIndex<<DB as sqlx::Database>::Row>,
//     // 'q: 'e,
//     // 'c: 'e,
//     // E: 'e + sqlx::Executor<'c, Database = DB>,
//     // DB: 'e + sqlx::Database,
//     // <DB as sqlx::database::HasArguments<'e>>::Arguments: sqlx::IntoArguments<'e, DB>,
//     // &'c str: sqlx::Encode<'c, DB> + sqlx::Decode<'c, DB> + sqlx::Type<DB>,
//     // String: sqlx::Encode<'c, DB> + sqlx::Decode<'c, DB> + sqlx::Type<DB>,
//     // DateTime<Utc>: sqlx::Encode<'c, DB> + sqlx::Decode<'c, DB> + sqlx::Type<DB>,
// {
//     let db_row: Option<UpdateWeatherStatusView> = sqlx::query_as(SELECT_UPDATE_VIEW_SQL)
//         .bind(update_id.clone())
//         // .try_map(|row| {
//         //     let update_id = row.try_get::<UpdateWeatherId, _>(0)?;
//         //     // let update_id: String = row.try_get(PRIMARY_KEY.clone())?;
//         //     // let update_id: UpdateWeatherId = UpdateWeatherId::for_labeled(update_id);
//         //
//         //     let state = row.try_get(STATE_COL.clone())?;
//         //
//         //     let ls_rep: String = row.try_get(UPDATE_STATUSES_COL.clone())?;
//         //     let status =
//         //         serde_json::from_str(&ls_rep).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;
//         //
//         //     let last_updated_at = row.try_get(LAST_UPDATED_AT_COL.clone())?;
//         //
//         //     Ok(UpdateWeatherStatusView {
//         //         update_id,
//         //         state,
//         //         update_statuses: status,
//         //         last_updated_at,
//         //     })
//         //
//         // })
//         .fetch_optional(executor)
//         .await?;
//         // .map_err(|err| err.into())?;
//
//     Ok(db_row)
//     // match db_row {
//     //     None => Ok(None),
//     //     Some(row) => {
//     //         let update_id: String = row.try_get(PRIMARY_KEY.clone())?;
//     //         let update_id = UpdateWeatherId::for_labeled(update_id);
//     //
//     //         let state = row.try_get(STATE_COL.clone())?;
//     //
//     //         let ls_rep: String = row.try_get(UPDATE_STATUSES_COL.clone())?;
//     //         let update_statuses = serde_json::from_str(&ls_rep)?;
//     //
//     //         let last_updated_at = row.try_get(LAST_UPDATED_AT_COL.clone())?;
//     //
//     //         Ok(Some(UpdateWeatherStatusView {
//     //             update_id,
//     //             state,
//     //             update_statuses,
//     //             last_updated_at,
//     //         }))
//     //     },
//     // }
//
//     // sqlx::query_as(STATUS_BY_ID_SQL.as_str())
//     //     .bind(update_id)
//     //     .fetch_optional(executor)
//     //     .await
//     //     .map_err(|err| err.into())
// }

#[derive(Debug)]
pub struct UpdateWeatherHistoryProjection {
    query: StreamQuery<WeatherEvent>,
    pool: PgPool,
}

impl UpdateWeatherHistoryProjection {
    pub async fn new(pool: PgPool) -> Result<Self, sqlx::Error> {
        static CREATE_TABLE_DDL: OnceCell<String> = OnceCell::new();
        let sql = CREATE_TABLE_DDL.get_or_init(|| {
            format!(
                r#"
                CREATE TABLE IF NOT EXISTS {table} (
                    {primary_key} TEXT PRIMARY KEY,
                    {state} TEXT,
                    {update_statuses} JSONB,
                    {last_updated_at} TIMESTAMPTZ NULL DEFAULT clock_timestamp()
                "#,
                table = UPDATE_WEATHER_HISTORY_TABLE.as_str(),
                primary_key = PRIMARY_KEY.as_str(),
                state = STATE_COL.as_str(),
                update_statuses = UPDATE_STATUSES_COL.as_str(),
                last_updated_at = LAST_UPDATED_AT_COL.as_str(),
            )
        });

        sqlx::query(sql).execute(&pool).await?;
        Ok(Self { query: query(None), pool })
    }
}

#[async_trait]
impl EventListener<WeatherEvent> for UpdateWeatherHistoryProjection {
    type Error = UpdateWeatherError;

    fn id(&self) -> &'static str {
        &UPDATE_WEATHER_HISTORY_TABLE
    }

    fn query(&self) -> &StreamQuery<WeatherEvent> {
        &self.query
    }

    async fn handle(&self, event: PersistedEvent<WeatherEvent>) -> Result<(), Self::Error> {
        use WeatherEvent as E;

        let mut tx = sqlx::Acquire::begin(&self.pool).await?;

        let event = event.into_inner();
        let event_t = event.clone();
        let view = self.fetch_optional_view(event.update_id()).await?;

        let result: Result<PgQueryResult, UpdateWeatherError> = match (event, view) {
            (E::UpdateStarted { update_id, zones }, None) => {
                Self::started(update_id, zones, &mut tx).await
            },
            (E::UpdateStarted { update_id, zones }, Some(view)) => {
                warn!(
                    restart_update=%update_id, restart_zones=?zones, previous_update=?view,
                    "unexpected update weather RESTART"
                );
                Self::started(update_id, zones, &mut tx).await
            },
            (event, Some(mut view)) => {
                let new_state = view.update_statuses.mutate(event);
                let update_state = if view.state != new_state { Some(new_state) } else { None };

                Self::update_or_insert(
                    view.update_id,
                    update_state,
                    Some(view.update_statuses),
                    &mut tx,
                )
                .await
            },
            (event, None) => {
                warn!(
                    ?event,
                    "unexpected update weather event without existing status"
                );

                let update_id = event.update_id().clone();
                let mut status = WeatherUpdateStatus::new(event.zones());
                let state = status.mutate(event);

                Self::update_or_insert(update_id, Some(state), Some(status), &mut tx).await
            },
        };

        // let result: PgQueryResult = match event.into_inner() {
        //     E::UpdateStarted { update_id, zones } => {
        //         Self::started(update_id, zones, &mut *tx).await?
        //     },
        //     // E::UpdateCompleted { update_id } | E::UpdateFailed { update_id } => {
        //     //     Some(Self::finished(update_id, &mut *tx).await?)
        //     // },
        //     // E::UpdateFailed { update_id } => Some(Self::failed(update_id, &mut *tx).await?),
        //     E::ObservationUpdated { update_id, zone, .. } => Some(
        //         Self::advance_zone_step(update_id, zone, UpdateStep::Observation, &mut *tx).await?
        //     ),
        //     E::ForecastUpdated { update_id, zone, .. } => Some(
        //         Self::advance_zone_step(update_id, zone, UpdateStep::Forecast, &mut *tx).await?
        //     ),
        //     E::AlertActivated { update_id, zone, .. } | E::AlertDeactivated { update_id, zone } => {
        //         Self::advance_zone_step(update_id, zone, UpdateStep::Alert, &mut *tx).await?
        //     },
        //     E::UpdateLocationFailed { update_id, zone, cause } => {
        //         Self::zone_update_failed(update_id, zone, &cause, &mut *tx).await?
        //     },
        //     E::AlertsReviewed { update_id } => {
        //         Self::alert_reviewed(update_id, &mut *tx).await?
        //     },
        // };

        let outcome = tx.commit().await;
        if let Err(ref error) = outcome {
            error!(
                event=?event_t,
                "postgres projection failed to commit update weather event transaction: {error:?}"
            );
        }

        debug!(
            event=?event_t,
            "update weather projection postgres query result: {result:?}"
        );
        outcome.map_err(|err| err.into())
    }
}

const SELECT_UPDATE_VIEW_SQL: &str = r#"
    SELECT update_id, state, update_statuses, last_updated_at
    FROM update_weather_history
    WHERE update_id = $1
    LIMIT 1
"#;

impl UpdateWeatherHistoryProjection {
    async fn started(
        update_id: UpdateWeatherId, zones: Vec<LocationZoneCode>, tx: &mut PgConnection,
    ) -> Result<PgQueryResult, UpdateWeatherError> {
        Self::update_or_insert(
            update_id,
            Some(UpdateWeatherStateDiscriminants::Active),
            Some(WeatherUpdateStatus::new(zones)),
            tx,
        )
        .await
    }

    async fn fetch_optional_view(
        &self, update_id: &UpdateWeatherId,
    ) -> Result<Option<UpdateWeatherStatusView>, UpdateWeatherError> {
        sqlx::query_as(SELECT_UPDATE_VIEW_SQL)
            .bind(update_id.clone())
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| err.into())
    }

    // async fn finished(
    //     update_id: UpdateWeatherId, tx: &mut PgConnection,
    // ) -> Result<PgQueryResult, UpdateWeatherError> {
    //     todo!()
    // }

    // async fn failed(
    //     update_id: UpdateWeatherId, zones: Vec<LocationZoneCode>, tx: &mut PgConnection,
    // ) -> Result<PgQueryResult, UpdateWeatherError> {
    //     todo!()
    // }

    // async fn advance_zone_step(
    //     update_id: UpdateWeatherId, zone: LocationZoneCode, step: UpdateStep,
    //     tx: &mut PgConnection,
    // ) -> Result<PgQueryResult, UpdateWeatherError> {
    //     do_fetch_optional_view(&update_id, tx)
    //         .await?
    //         .map(|mut view| {
    //             let foo = view.status
    //         })
    //
    //     todo!()
    // }

    // async fn zone_update_failed(
    //     update_id: UpdateWeatherId, zone: LocationZoneCode, cause: &str,
    //     tx: &mut PgConnection,
    // ) -> Result<PgQueryResult, UpdateWeatherError> {
    //     todo!()
    // }
}

// static UPDATE_BASE_SQL: Lazy<sql::Update> = Lazy::new(|| {
//     sql::Update::new()
//         .update(&UPDATE_WEATHER_HISTORY_TABLE)
//         .where_clause(format!("{PRIMARY_KEY} = $1").as_str())
// });

impl UpdateWeatherHistoryProjection {
    async fn update_or_insert(
        update_id: UpdateWeatherId, state: Option<UpdateWeatherStateDiscriminants>,
        update_status: Option<WeatherUpdateStatus>, tx: &mut PgConnection,
    ) -> Result<PgQueryResult, UpdateWeatherError> {
        let update_clause = sql::Update::new().set(
            format!(
                "{last_updated_at} = EXCLUDED.{last_updated_at}",
                last_updated_at = LAST_UPDATED_AT_COL.as_str(),
            )
            .as_str(),
        );

        let update_clause = if state.is_some() {
            update_clause.set(
                format!(
                    "{state_col} = EXCLUDED.{state_col}",
                    state_col = STATE_COL.as_str()
                )
                .as_str(),
            )
        } else {
            update_clause
        };

        let update_clause = if update_status.is_some() {
            update_clause.set(
                format!(
                    "{update_statuses_col} = EXCLUDED.{update_statuses_col}",
                    update_statuses_col = UPDATE_STATUSES_COL.as_str(),
                )
                .as_str(),
            )
        } else {
            update_clause
        };

        let sql = Self::build_insert(update_clause);

        let update_status_json = update_status.map(serde_json::to_value).transpose()?;
        sqlx::query(&sql)
            .bind(update_id)
            .bind(state)
            .bind(update_status_json)
            .bind(Utc::now())
            .execute(tx)
            .await
            .map_err(|err| err.into())
    }

    fn build_insert(update_clause: sql::Update) -> String {
        let conflict_clause = format!(
            "( {primary_key} ) DO UPDATE {update_clause}",
            primary_key = PRIMARY_KEY.as_str()
        );

        sql::Insert::new()
            .insert_into(
                format!(
                    "{table} ( {columns} )",
                    table = UPDATE_WEATHER_HISTORY_TABLE.as_str(),
                    columns = COLUMNS_REP.as_str(),
                )
                .as_str(),
            )
            .values(&VALUES_REP)
            .on_conflict(conflict_clause.as_str())
            .to_string()
    }
}
