use anyhow::anyhow;
use clap::Parser;
use settings_loader::{LoadingOptions, SettingsLoader};
use tokio_util::task::TaskTracker;
use tracing::log::Level;
use weather_disintegrate::model::registrar::protocol::RegistrarEvent;
use weather_disintegrate::model::weather::WeatherEvent;
use weather_disintegrate::server::{self, AppState};
use weather_disintegrate::{CliOptions, Settings};

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

    let task_tracker = TaskTracker::new();

    let app_state = AppState::new(&settings, &task_tracker).await?;

    setup_event_tracing(&app_state, &task_tracker);
    task_tracker.spawn(async move { http_server(app_state, &settings).await });

    task_tracker.close();
    task_tracker.wait().await;

    Ok(())
}

async fn http_server(app_state: AppState, settings: &Settings) -> anyhow::Result<()> {
    let server = server::Server::build(app_state, settings).await?;
    tracing::info!(?server, "starting server...");
    server.run_until_stopped().await.map_err(|err| err.into())
}

#[tracing::instrument(level = "debug", ret, err)]
pub fn load_settings(options: &CliOptions) -> anyhow::Result<weather_disintegrate::Settings> {
    let app_environment = std::env::var(CliOptions::env_app_environment()).ok();
    if app_environment.is_none() {
        tracing::info!("No environment configuration override provided.");
    }

    weather_disintegrate::Settings::load(options).map_err(|err| err.into())
}

fn setup_event_tracing(app: &AppState, task_tracker: &TaskTracker) {
    let registrar_es = app.registrar_support.event_store.clone();
    task_tracker.spawn(async move {
        let registrar_tracing =
            weather_disintegrate::model::TracingProcessor::<RegistrarEvent>::new(
                "registrar",
                Level::Info,
            );

        disintegrate_postgres::PgEventListener::builder(registrar_es)
            .register_listener(
                registrar_tracing,
                disintegrate_postgres::PgEventListenerConfig::poller(
                    std::time::Duration::from_millis(50),
                ),
            )
            .start_with_shutdown(weather_disintegrate::shutdown())
            .await
            .map_err(|e| anyhow!("registrar tracing event listener exited with error: {e}"))?;
        Ok::<(), anyhow::Error>(())
    });

    let weather_es = app.weather_support.event_store.clone();
    task_tracker.spawn(async move {
        let weather_tracing = weather_disintegrate::model::TracingProcessor::<WeatherEvent>::new(
            "weather",
            Level::Info,
        );

        disintegrate_postgres::PgEventListener::builder(weather_es)
            .register_listener(
                weather_tracing,
                disintegrate_postgres::PgEventListenerConfig::poller(
                    std::time::Duration::from_millis(50),
                ),
            )
            .start_with_shutdown(weather_disintegrate::shutdown())
            .await
            .map_err(|e| anyhow!("weather tracing event listener exited with error: {e}"))?;

        Ok::<(), anyhow::Error>(())
    });
}
