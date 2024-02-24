mod cli_options;
mod http_api_settings;
#[cfg(test)]
mod tests;

pub use cli_options::CliOptions;
pub use http_api_settings::HttpApiSettings;

use settings_loader::common::database::DatabaseSettings;
use settings_loader::SettingsLoader;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Settings {
    pub http_api: HttpApiSettings,
    pub database: DatabaseSettings,
    pub registrar: DomainSettings,
    pub weather: DomainSettings,
    // pub zone: AggregateSettings,
    // pub update_locations: AggregateSettings,
}

impl SettingsLoader for Settings {
    type Options = CliOptions;
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct DomainSettings;

// #[derive(Debug, Clone, PartialEq, Deserialize)]
// pub struct AggregateSettings {
//     #[serde(default = "PostgresStorageConfig::default_event_journal_table")]
//     pub event_journal_table_name: TableName,
//
//     #[serde(default = "PostgresStorageConfig::default_snapshot_table")]
//     pub snapshots_table_name: TableName,
//
//     #[serde(default = "PostgresStorageConfig::default_projection_offsets_table")]
//     pub projection_offsets_table_name: TableName,
// }

// impl Default for AggregateSettings {
//     fn default() -> Self {
//         Self {
//             event_journal_table_name: PostgresStorageConfig::default_event_journal_table(),
//             snapshots_table_name: PostgresStorageConfig::default_snapshot_table(),
//             projection_offsets_table_name: PostgresStorageConfig::default_projection_offsets_table(
//             ),
//         }
//     }
// }

// pub fn storage_config_from(
//     db: &DatabaseSettings, aggregate: &AggregateSettings,
// ) -> PostgresStorageConfig {
//     PostgresStorageConfig {
//         key_prefix: Default::default(),
//         username: db.username.clone(),
//         password: db.password.clone(),
//         host: db.host.clone(),
//         port: db.port,
//         database_name: db.database_name.clone(),
//         event_journal_table_name: aggregate.event_journal_table_name.clone(),
//         projection_offsets_table_name: aggregate.projection_offsets_table_name.clone(),
//         snapshot_table_name: aggregate.snapshots_table_name.clone(),
//         require_ssl: db.require_ssl,
//         min_connections: db.min_connections,
//         max_connections: db.max_connections,
//         max_lifetime: db.max_lifetime,
//         acquire_timeout: db.acquire_timeout,
//         idle_timeout: db.idle_timeout,
//     }
// }
