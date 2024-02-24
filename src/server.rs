mod api_errors;
mod api_result;
mod health_routes;
mod state;
mod weather_routes;

pub use crate::server::state::AppState;

use crate::server::api_errors::ApiError;
use crate::settings::HttpApiSettings;
use crate::Settings;
use axum::http::{HeaderValue, Request, Response, StatusCode, Uri};
use axum::{BoxError, Router};
use settings_loader::common::database::DatabaseSettings;
use sqlx::PgPool;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, TcpListener};
use tokio::signal;
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::{KeyExtractor, SmartIpKeyExtractor};
use tower_governor::GovernorError;
// use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse};
use tower_http::ServiceBuilderExt;
use utoipa::OpenApi;
use utoipa_swagger_ui::{SwaggerUi, Url as SwaggerUrl};

pub type HttpJoinHandle = JoinHandle<Result<(), ApiError>>;

pub struct Server {
    port: u16,
    server_handle: HttpJoinHandle,
}

impl fmt::Debug for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Server").field("port", &self.port).finish()
    }
}

impl Server {
    #[instrument(level = "debug", skip(settings), err)]
    pub async fn build(state: AppState, settings: &Settings) -> Result<Self, ApiError> {
        let address = settings.http_api.server.address();
        let listener = tokio::net::TcpListener::bind(&address)
            .await
            .map_err(|err| ApiError::Bootstrap(err.into()))?;
        tracing::info!(
            "{:?} API listening on {address}: {listener:?}",
            std::env::current_exe()
        );
        let std_listener = listener.into_std().map_err(|err| ApiError::Bootstrap(err.into()))?;
        let port = std_listener
            .local_addr()
            .map_err(|err| ApiError::Bootstrap(err.into()))?
            .port();

        let run_params = RunParameters::from_settings(settings);
        let server_handle = run_http_server(std_listener, state, &run_params).await?;
        Ok(Self { port, server_handle })
    }

    pub const fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), ApiError> {
        self.server_handle.await?
    }
}

pub fn get_connection_pool(settings: &DatabaseSettings) -> PgPool {
    let connection_options = settings.pg_connect_options_with_db();
    settings.pg_pool_options().connect_lazy_with(connection_options)
}

#[derive(Debug, Clone)]
pub struct RunParameters {
    pub http_api: HttpApiSettings,
}

impl RunParameters {
    pub fn from_settings(settings: &Settings) -> Self {
        Self { http_api: settings.http_api.clone() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MyKeyExtractor;

impl KeyExtractor for MyKeyExtractor {
    type Key = IpAddr;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        Ok(SmartIpKeyExtractor.extract(req).unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)))
    }
}

#[derive(Copy, Clone)]
struct MyMakeRequestId;

impl tower_http::request_id::MakeRequestId for MyMakeRequestId {
    fn make_request_id<B>(
        &mut self, _request: &Request<B>,
    ) -> Option<tower_http::request_id::RequestId> {
        let request_id = HeaderValue::from_str(::cuid2::create_id().as_str()).unwrap();
        Some(tower_http::request_id::RequestId::new(request_id))
    }
}

#[instrument(level = "debug", err)]
pub async fn run_http_server(
    listener: TcpListener, state: AppState, params: &RunParameters,
) -> Result<HttpJoinHandle, ApiError> {
    let governor_conf = Box::new(
        GovernorConfigBuilder::default()
            .burst_size(params.http_api.rate_limit.burst_size)
            .period(params.http_api.rate_limit.per_duration)
            .key_extractor(MyKeyExtractor)
            .finish()
            .unwrap(),
    );

    let middleware_stack = ServiceBuilder::new()
        .layer(tower_governor::GovernorLayer {
            config: Box::leak(governor_conf), // okay to leak because it is created once and then used by layer
        })
        .layer(axum::error_handling::HandleErrorLayer::new(
            handle_api_error,
        ))
        .timeout(params.http_api.timeout)
        .compression()
        .trace_for_http()
        // .layer(tower_http::compression::CompressionLayer::new())
        // .layer(
        //     tower_http::trace::TraceLayer::new_for_http()
        //         .make_span_with(DefaultMakeSpan::new().include_headers(true))
        //         .on_response(DefaultOnResponse::new().include_headers(true)),
        // )
        .set_x_request_id(MyMakeRequestId)
        .propagate_x_request_id();

    let api_routes = Router::new()
        .nest("/health", health_routes::api())
        .nest("/weather", weather_routes::api())
        .with_state(state);

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").urls(vec![
            (
                SwaggerUrl::with_primary("weather_api", "/api-doc/weather-openapi.json", true),
                weather_routes::WeatherApiDoc::openapi(),
            ),
            (
                SwaggerUrl::with_primary("health_api", "/api-doc/health-openapi.json", true),
                health_routes::HealthApiDoc::openapi(),
            ),
        ]))
        .nest("/api/v1", api_routes)
        .fallback(fallback)
        .layer(middleware_stack);

    let listener_t = tokio::net::TcpListener::from_std(listener)?;
    let handle = tokio::spawn(async move {
        debug!(app_routes=?app, "starting API server...");
        let server = axum::serve(listener_t, app.into_make_service());

        // let builder = axum::serve::Server::from_tcp(listener)?;
        // let server = builder.serve(app.into_make_service());
        let graceful = server.with_graceful_shutdown(shutdown_signal());
        graceful.await?;
        info!("{:?} API shutting down", std::env::current_exe());
        Ok(())
    });

    Ok(handle)
}

async fn fallback(uri: Uri) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("No route found for {uri}"))
}

async fn handle_api_error(error: BoxError) -> Response<String> {
    if error.is::<tower::timeout::error::Elapsed>() {
        let response = Response::new(format!("REQUEST TIMEOUT: {error}"));
        let (mut parts, body) = response.into_parts();
        parts.status = StatusCode::REQUEST_TIMEOUT;
        Response::from_parts(parts, body)
    // } else if error.is::<tower_governor::errors::GovernorError>() {
    //     tower_governor::errors::display_error(error)
    } else {
        let response = Response::new(format!("INTERNAL SERVER ERROR: {error}"));
        let (mut parts, body) = response.into_parts();
        parts.status = StatusCode::INTERNAL_SERVER_ERROR;
        Response::from_parts(parts, body)
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install terminate signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("signal received, starting graceful shutdown");
}
