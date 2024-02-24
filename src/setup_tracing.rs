use once_cell::sync::Lazy;
use tracing::Subscriber;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};

pub static TEST_TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info";
    let subscriber_name = "test";
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub fn get_tracing_subscriber(log_directives: impl AsRef<str>) -> impl Subscriber + Sync + Send {
    // let console = console_subscriber::Builder::spawn(); //console_subscriber::ConsoleLayer::builder().spawn();
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_directives.as_ref()));
    let bunyan_formatting = tracing_bunyan_formatter::BunyanFormattingLayer::new(
        std::env::current_exe()
            .expect("failed to identify name of application")
            .to_string_lossy()
            .to_string(),
        std::io::stdout,
    );

    Registry::default()
        // .with(console)
        // .with(tracing_subscriber::fmt::layer())
        .with(env_filter)
        .with(tracing_bunyan_formatter::JsonStorageLayer)
        .with(bunyan_formatting)
}

/// Compose multiple layers into a `tracing`'s subscriber.
///
/// # Implementation Notes
///
/// We are using `impl Subscriber` as return type to avoid having to spell out the actual type of
/// the returned subscriber, which is indeed quite complex.
/// We need to explicitly call out that returned subscriber is `Send` and `Sync` to make it
/// possible to pass it to `init_subscriber` later on.
pub fn get_subscriber<S0, S1, W>(name: S0, env_filter: S1, sink: W) -> impl Subscriber + Sync + Send
where
    S0: Into<String>,
    S1: AsRef<str>,
    W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    // let (flame_subscriber, _guard) = FlameLayer::with_file("./tracing.folded").unwrap();

    let formatting_layer = BunyanFormattingLayer::new(name.into(), sink);

    Registry::default()
        .with(env_filter)
        // .with(flame_subscriber)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// Register a subscriber as global default to process span data.
///
/// It should be only called once!
pub fn init_subscriber(subscriber: impl Subscriber + Sync + Send) {
    LogTracer::init().expect("Failed to set logger");
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");
}
