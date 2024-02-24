use clap::Parser;
use settings_loader::{LoadingOptions, SettingsLoader};
use weather_disintegrate::server::{self, AppState};
use weather_disintegrate::CliOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = weather_disintegrate::setup_tracing::get_tracing_subscriber("info");
    weather_disintegrate::setup_tracing::init_subscriber(subscriber);

    let options = CliOptions::parse();
    if options.secrets.is_none() {
        tracing::warn!("No secrets configuration provided. Passwords (e.g., for the database) should be confined to a secret configuration and sourced in a secure manner.");
    }
    let settings = load_settings(&options);
    tracing::info!("settings = {settings:?}");
    let settings = settings?;

    let app_state = AppState::new(&settings).await?;

    let server = server::Server::build(app_state, &settings).await?;
    tracing::info!(?server, "starting server...");
    server.run_until_stopped().await.map_err(|err| err.into())
}

#[tracing::instrument(level = "debug", ret, err)]
fn load_settings(options: &CliOptions) -> anyhow::Result<weather_disintegrate::Settings> {
    let app_environment = std::env::var(CliOptions::env_app_environment()).ok();
    if app_environment.is_none() {
        tracing::info!("No environment configuration override provided.");
    }

    weather_disintegrate::Settings::load(options).map_err(|err| err.into())
}
