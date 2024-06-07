use std::{convert::Infallible, future::ready, time::{Duration, Instant}};

use axum::{
    body::{Body, Bytes},
    extract::{MatchedPath, Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Method},
    Json,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tower_http::{
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use serde_json::{json, Value};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use tracing::Span;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod http_metrics;

pub(crate) const PKG_NAME: &str = concat!("", env!("CARGO_PKG_NAME"));
pub(crate) const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

#[derive(Debug)]
pub struct ServerConfig {
    pub port: u16,
    pub req_timeout: u8,
    pub metrics_port: u16,
    pub log_level: String,
}

pub fn run_server(cfg: ServerConfig) {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}={},tower_http=debug", PKG_NAME, cfg.log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // creates a new default tokio multi-thread [Runtime](tokio::runtime::Runtime) with all features enabled
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime")
        .block_on(async {
            // The `/metrics` endpoint should not be publicly available. If behind a reverse proxy, this
            // can be achieved by rejecting requests to `/metrics`. In this example, a second server is
            // started on another port to expose `/metrics`.
            let (_main_server, _metrics_server) = tokio::join!(
                start_main_server(&cfg),
                http_metrics::start_metrics_server(&cfg),
            );
        });
}

async fn start_main_server(cfg: &ServerConfig) {
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/version", get(|| async { Json(json!({ "version": VERSION })) }))
        // Add some logging so we can see the streams going through
        .route_layer(middleware::from_fn(http_metrics::track_request_metrics))
        .layer((
            TraceLayer::new_for_http().on_body_chunk(|chunk: &Bytes, _latency: Duration, _span: &Span| tracing::debug!("streaming {} bytes", chunk.len())),
            TimeoutLayer::new(Duration::from_secs(cfg.req_timeout.into())),
        ));
        // .with_state(client);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", cfg.port))
        .await
        .unwrap();
    tracing::info!("running tsumori-bridge server on {}...", listener.local_addr().unwrap());
    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await.unwrap();
}

// support graceful shutdown
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn it_works() {
    //     let result = add(2, 2);
    //     assert_eq!(result, 4);
    // }
}
