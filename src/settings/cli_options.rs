use clap::Parser;
use config::builder::DefaultState;
use config::ConfigBuilder;
use settings_loader::{Environment, LoadingOptions, SettingsError};
use std::path::PathBuf;

#[derive(Debug, Default, Parser, PartialEq, Eq)]
#[clap(author, version, about)]
pub struct CliOptions {
    /// Explicit configuration to load, bypassing inferred configuration load mechanism. If this
    /// option is used, the application + environment will be ignored; however, secrets, env var,
    /// and explicit overrides will still be used.
    ///
    /// Default behavior is to infer-load configuration based on `APP_ENVIRONMENT` envvar.
    #[clap(short, long, value_name = "PATH_TO_CONFIG_FILE")]
    pub config: Option<PathBuf>,

    /// specify path to secrets configuration file
    #[clap(long, value_name = "PATH_TO_SECRETS_FILE")]
    pub secrets: Option<PathBuf>,

    /// specify the environment configuration override used in inferred configuration load.
    #[clap(short = 'e', long = "env")]
    pub environment: Option<Environment>,

    /// Override filesystem path used to search for application and environment configuration files.
    /// Directories are separated by the ':' character.
    /// Default path is "./resources".
    #[clap(short = 's', long = "search-path", value_name = "SETTINGS_SEARCH_PATH")]
    pub settings_search_path: Option<String>,
    // /// Specify the machine id [0, 31) used in correlation id generation, overriding what may be set
    // /// in an environment variable. This id should be unique for the entity type within a cluster
    // /// environment. Different entity types can use the same machine id.
    // /// Optionally overrides the engine.machine_id setting.
    // #[clap(short, long, value_name = "[0, 31)")]
    // pub machine_id: Option<i8>,

    // /// Specify the node id [0, 31) used in correlation id generation, overriding what may be set
    // /// in an environment variable. This id should be unique for the entity type within a cluster
    // /// environment. Different entity types can use the same machine id.
    // /// Optionally override the engine.node_id setting.
    // #[clap(short, long, value_name = "[0, 31)")]
    // pub node_id: Option<i8>,
}

const DEFAULT_SEARCH_PATH: &str = "./resources";

impl LoadingOptions for CliOptions {
    type Error = SettingsError;

    fn config_path(&self) -> Option<PathBuf> {
        self.config.clone()
    }

    fn secrets_path(&self) -> Option<PathBuf> {
        self.secrets.clone()
    }

    fn implicit_search_paths(&self) -> Vec<PathBuf> {
        let search_path = self.settings_search_path.as_deref().unwrap_or(DEFAULT_SEARCH_PATH);
        search_path.split(':').map(PathBuf::from).collect()
    }

    fn load_overrides(
        &self, config: ConfigBuilder<DefaultState>,
    ) -> Result<ConfigBuilder<DefaultState>, Self::Error> {
        // let config = match self.machine_id {
        //     None => config,
        //     Some(machine_id) => config.set_override("machine_id", i64::from(machine_id))?,
        // };

        // let config = match self.node_id {
        //     None => config,
        //     Some(node_id) => config.set_override("node_id", i64::from(node_id))?,
        // };

        Ok(config)
    }

    fn environment_override(&self) -> Option<Environment> {
        self.environment.clone()
    }
}
